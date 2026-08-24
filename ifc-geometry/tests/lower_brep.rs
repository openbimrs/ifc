//! Faceted B-rep lowering against the real corpus.
//!
//! # Why topology assertions and not a triangle count
//!
//! A brep that lowered to the right number of triangles can still be wrong:
//! duplicated vertices, unshared edges, and dropped holes all survive a
//! naive count. These tests assert on the manifold structure itself, using
//! numbers read directly out of the fixture records.

use axiolid_model::GeometryNode;
use axiolid_topology::Orientation;
use ifc_geometry::lower::{lower_faceted_brep_node, LoweringSession, Tolerance};
use ifc_geometry::transform::Transform;
use ifc_geometry::units;
use ifc_model::{Codec, Entity, EntityId, Model, Value};
use ifc_step::StepCodec;
use std::path::PathBuf;

fn fixture(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../test/fixtures")
        .join(rel)
}

fn model_of(rel: &str) -> Model {
    StepCodec.read_path(&fixture(rel)).expect("fixture parses")
}

fn brep_of(
    graph: &axiolid_model::GeometryGraph,
    root: axiolid_model::NodeId,
) -> &axiolid_topology::BRep<axiolid_model::NodeId> {
    match graph.get(root).expect("root resolves") {
        GeometryNode::BRep(brep) => brep,
        other => panic!("expected a BRep node, got {other:?}"),
    }
}

/// The cube fixture lowers to exactly the topology its records describe.
///
/// Ground truth read directly out of `issue_1985_scaled_kinds.ifc`:
///
/// ```text
/// #20..#27  eight IFCCARTESIANPOINTs, the cube corners
/// #28..#45  six IFCPOLYLOOPs of four points, each in an IFCFACEOUTERBOUND
/// #46= IFCCLOSEDSHELL((#30,#33,#36,#39,#42,#45))
/// #47= IFCFACETEDBREP(#46)
/// ```
///
/// A cube is the Euler characteristic check everyone knows: V - E + F = 2,
/// so 8 - 12 + 6 = 2. Getting 24 vertices or 24 edges means interning failed
/// and the "solid" is really six disconnected quads.
#[test]
fn the_cube_fixture_lowers_to_eight_vertices_and_twelve_edges() {
    let model = model_of("ifclite-geometry/issue_1985_scaled_kinds.ifc");
    let scale = units::resolve(&model);
    let id = *model
        .ids_of_type("IFCFACETEDBREP")
        .first()
        .expect("the fixture contains a faceted brep");

    let mut session = LoweringSession::new(&model, &scale, Tolerance::building_scale());
    let node = lower_faceted_brep_node(&mut session, id, Transform::identity())
        .expect("the cube must lower");
    let lowered = session.finish(node).expect("session finishes");
    let brep = brep_of(&lowered.graph, lowered.root);

    assert_eq!(brep.vertices().len(), 8, "eight distinct corner points");
    assert_eq!(brep.edges().len(), 12, "a cube has twelve shared edges");
    assert_eq!(brep.faces().len(), 6, "six faces");
    assert_eq!(brep.loops().len(), 6, "one loop per face");
    assert_eq!(brep.shells().len(), 1, "one closed shell");
    assert_eq!(brep.solids().len(), 1, "one solid");

    let euler =
        brep.vertices().len() as i64 - brep.edges().len() as i64 + brep.faces().len() as i64;
    assert_eq!(euler, 2, "V - E + F must be 2 for a closed cube");
}

/// Every loop closes: the edge from the last point back to the first exists.
///
/// The schema lists N points and leaves the closing edge implicit. Emitting
/// N-1 edges leaves a gap that no vertex count would reveal.
#[test]
fn every_loop_is_closed_with_as_many_edges_as_points() {
    let model = model_of("ifclite-geometry/issue_1985_scaled_kinds.ifc");
    let scale = units::resolve(&model);
    let id = *model.ids_of_type("IFCFACETEDBREP").first().expect("brep");

    let mut session = LoweringSession::new(&model, &scale, Tolerance::building_scale());
    let node = lower_faceted_brep_node(&mut session, id, Transform::identity()).expect("lowers");
    let lowered = session.finish(node).expect("finishes");
    let brep = brep_of(&lowered.graph, lowered.root);

    for (index, wire) in brep.loops().iter().enumerate() {
        assert_eq!(
            wire.edges.len(),
            4,
            "loop {index}: each cube face is a quad, so four edges including the closing one"
        );
    }
}

/// Interior edges are shared by exactly two faces, which is what makes it a solid.
///
/// Every edge of a closed manifold is used twice, once in each direction. If
/// interning keyed on the ordered pair instead of the unordered one, each edge
/// would be used once and the count below would be 1 everywhere.
#[test]
fn each_edge_is_used_by_exactly_two_faces_in_opposite_senses() {
    let model = model_of("ifclite-geometry/issue_1985_scaled_kinds.ifc");
    let scale = units::resolve(&model);
    let id = *model.ids_of_type("IFCFACETEDBREP").first().expect("brep");

    let mut session = LoweringSession::new(&model, &scale, Tolerance::building_scale());
    let node = lower_faceted_brep_node(&mut session, id, Transform::identity()).expect("lowers");
    let lowered = session.finish(node).expect("finishes");
    let brep = brep_of(&lowered.graph, lowered.root);

    let mut forward = vec![0usize; brep.edges().len()];
    let mut reversed = vec![0usize; brep.edges().len()];
    for wire in brep.loops() {
        for use_of in &wire.edges {
            match use_of.orientation {
                Orientation::Forward => forward[use_of.edge.index()] += 1,
                Orientation::Reversed => reversed[use_of.edge.index()] += 1,
            }
        }
    }

    for edge in 0..brep.edges().len() {
        assert_eq!(
            (forward[edge], reversed[edge]),
            (1, 1),
            "edge {edge} must be walked once forward and once backward"
        );
    }
}

/// Vertex positions are unit-converted and frame-placed.
///
/// The fixture declares MILLI metre and its cube spans 0..1 in file units, so
/// the lowered solid spans 0..0.001 metres. Skipping the conversion yields a
/// cube 1000x too large -- a mistake that renders fine and measures wrong.
#[test]
fn vertex_positions_are_converted_to_metres_and_placed() {
    let model = model_of("ifclite-geometry/issue_1985_scaled_kinds.ifc");
    let scale = units::resolve(&model);
    let id = *model.ids_of_type("IFCFACETEDBREP").first().expect("brep");

    let offset = Transform::translation([10.0, 0.0, 0.0]);
    let mut session = LoweringSession::new(&model, &scale, Tolerance::building_scale());
    let node = lower_faceted_brep_node(&mut session, id, offset).expect("lowers");
    let lowered = session.finish(node).expect("finishes");
    let brep = brep_of(&lowered.graph, lowered.root);

    let xs: Vec<f64> = brep.vertices().iter().map(|v| v.position.x).collect();
    let min = xs.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = xs.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    assert!(
        (min - 10.0).abs() < 1e-12,
        "min x should be the frame origin 10.0, got {min}"
    );
    assert!(
        (max - 10.001).abs() < 1e-12,
        "1 mm span converts to 0.001 m, so max x is 10.001, got {max}"
    );
}

/// The 12-solid fixture keeps each body independent while sharing its point pool.
///
/// Ground truth: `shared_point_faceted_brep.ifc` holds 209 IFCCARTESIANPOINT
/// records, 2028 faces and 12 IFCFACETEDBREPs. The points are shared ACROSS
/// solids, but a vertex arena is per-solid: two bodies quoting the same point
/// are still two bodies. So every brep must be internally deduplicated while
/// no brep may contain another's topology.
#[test]
fn twelve_solids_each_intern_their_own_shared_points() {
    let model = model_of("ifclite-geometry/shared_point_faceted_brep.ifc");
    let scale = units::resolve(&model);
    let ids = model.ids_of_type("IFCFACETEDBREP").to_vec();
    assert_eq!(ids.len(), 12, "the fixture holds twelve solids");

    let mut total_faces = 0usize;
    for id in &ids {
        // One session per solid: this test is about per-body interning, and a
        // fresh graph makes the arena boundary explicit.
        let mut session = LoweringSession::new(&model, &scale, Tolerance::building_scale());
        let node = lower_faceted_brep_node(&mut session, *id, Transform::identity())
            .unwrap_or_else(|e| panic!("{id} must lower: {e}"));
        let lowered = session.finish(node).expect("session finishes");
        let brep = brep_of(&lowered.graph, lowered.root);

        assert_eq!(brep.solids().len(), 1, "{id}: exactly one solid per brep");
        assert!(
            brep.vertices().len() <= 209,
            "{id}: interning must not exceed the file's whole point pool, got {}",
            brep.vertices().len()
        );
        // A closed polyhedron: every face contributes >= 3 edge uses, and each
        // edge is shared by two faces, so E is well under 3F/2 * 2.
        assert!(
            brep.edges().len() < brep.faces().len() * 3,
            "{id}: {} edges for {} faces means edges are not being shared",
            brep.edges().len(),
            brep.faces().len()
        );
        total_faces += brep.faces().len();
    }
    assert_eq!(total_faces, 2028, "every face in the file is accounted for");
}

fn point(model: &mut Model, id: u64, xyz: [f64; 3]) {
    model.insert(
        EntityId(id),
        Entity::new(
            "IFCCARTESIANPOINT",
            vec![Value::List(xyz.iter().map(|v| Value::Real(*v)).collect())],
        ),
    );
}

fn refs(ids: &[u64]) -> Value {
    Value::List(ids.iter().map(|i| Value::Ref(EntityId(*i))).collect())
}

/// Build a tetrahedron shell from four corner points starting at `base`.
///
/// Returns the shell id. Four triangles, six edges, four vertices: the
/// smallest closed manifold, so a void needs no extra machinery to be real.
fn tetra_shell(model: &mut Model, base: u64, origin: [f64; 3], size: f64) -> u64 {
    let p = base;
    point(model, p, origin);
    point(model, p + 1, [origin[0] + size, origin[1], origin[2]]);
    point(model, p + 2, [origin[0], origin[1] + size, origin[2]]);
    point(model, p + 3, [origin[0], origin[1], origin[2] + size]);

    let corners = [
        [p, p + 2, p + 1],
        [p, p + 1, p + 3],
        [p, p + 3, p + 2],
        [p + 1, p + 2, p + 3],
    ];
    let mut faces = Vec::new();
    let mut next = base + 10;
    for corner in corners {
        model.insert(
            EntityId(next),
            Entity::new("IFCPOLYLOOP", vec![refs(&corner)]),
        );
        model.insert(
            EntityId(next + 1),
            Entity::new(
                "IFCFACEOUTERBOUND",
                vec![Value::Ref(EntityId(next)), Value::Bool(true)],
            ),
        );
        model.insert(
            EntityId(next + 2),
            Entity::new("IFCFACE", vec![refs(&[next + 1])]),
        );
        faces.push(next + 2);
        next += 3;
    }
    let shell = base + 90;
    model.insert(
        EntityId(shell),
        Entity::new("IFCCLOSEDSHELL", vec![refs(&faces)]),
    );
    shell
}

/// `IfcFacetedBrepWithVoids` keeps its void shells, which is what hollows it.
///
/// A hollow block and a solid block have identical outer boundaries. Dropping
/// the Voids attribute produces geometry that looks right and measures wrong,
/// so the void must survive into `Solid::voids`.
#[test]
fn a_brep_with_voids_keeps_its_interior_shells() {
    let mut model = Model::new();
    let outer = tetra_shell(&mut model, 100, [0.0, 0.0, 0.0], 10.0);
    let inner = tetra_shell(&mut model, 200, [1.0, 1.0, 1.0], 2.0);
    model.insert(
        EntityId(900),
        Entity::new(
            "IFCFACETEDBREPWITHVOIDS",
            vec![Value::Ref(EntityId(outer)), refs(&[inner])],
        ),
    );

    let scale = units::resolve(&model);
    let mut session = LoweringSession::new(&model, &scale, Tolerance::building_scale());
    let node = lower_faceted_brep_node(&mut session, EntityId(900), Transform::identity())
        .expect("a brep with voids must lower");
    let lowered = session.finish(node).expect("finishes");
    let brep = brep_of(&lowered.graph, lowered.root);

    assert_eq!(brep.shells().len(), 2, "outer shell plus one void shell");
    let solid = &brep.solids()[0];
    assert_eq!(
        solid.voids.len(),
        1,
        "the void must be recorded on the solid"
    );
    assert_ne!(solid.outer, solid.voids[0], "void is not the outer shell");
    assert_eq!(
        brep.vertices().len(),
        8,
        "two disjoint tetrahedra keep their own four corners each"
    );
}

/// A reversed `IfcFaceBound.Orientation` is preserved, not normalized away.
///
/// `.F.` means the loop bounds the face in the opposite sense. Forcing every
/// bound to Forward inverts such a face, which flips its normal and breaks any
/// downstream inside/outside test.
#[test]
fn a_reversed_bound_orientation_survives_lowering() {
    let mut model = Model::new();
    let shell = tetra_shell(&mut model, 100, [0.0, 0.0, 0.0], 5.0);
    // Flip the first face's bound: #111 is the first IFCFACEOUTERBOUND.
    model.insert(
        EntityId(111),
        Entity::new(
            "IFCFACEOUTERBOUND",
            vec![Value::Ref(EntityId(110)), Value::Bool(false)],
        ),
    );
    model.insert(
        EntityId(900),
        Entity::new("IFCFACETEDBREP", vec![Value::Ref(EntityId(shell))]),
    );

    let scale = units::resolve(&model);
    let mut session = LoweringSession::new(&model, &scale, Tolerance::building_scale());
    let node = lower_faceted_brep_node(&mut session, EntityId(900), Transform::identity())
        .expect("lowers");
    let lowered = session.finish(node).expect("finishes");
    let brep = brep_of(&lowered.graph, lowered.root);

    let reversed = brep
        .faces()
        .iter()
        .flat_map(|face| face.bounds.iter())
        .filter(|bound| bound.orientation == Orientation::Reversed)
        .count();
    assert_eq!(reversed, 1, "exactly the one flipped bound is Reversed");
}

/// A loop that collapses to fewer than three edges is rejected, not emitted.
///
/// Exporters do produce loops with repeated points. Such a loop bounds no
/// area; admitting it creates a face a boolean will later divide by zero on.
/// Failing loudly at read time names the entity while the context still exists.
#[test]
fn a_collapsed_loop_is_reported_as_degenerate() {
    let mut model = Model::new();
    let shell = tetra_shell(&mut model, 100, [0.0, 0.0, 0.0], 5.0);
    // Rewrite the first loop so all three slots are the same point.
    model.insert(
        EntityId(110),
        Entity::new("IFCPOLYLOOP", vec![refs(&[100, 100, 100])]),
    );
    model.insert(
        EntityId(900),
        Entity::new("IFCFACETEDBREP", vec![Value::Ref(EntityId(shell))]),
    );

    let scale = units::resolve(&model);
    let mut session = LoweringSession::new(&model, &scale, Tolerance::building_scale());
    let error = lower_faceted_brep_node(&mut session, EntityId(900), Transform::identity())
        .expect_err("a collapsed loop must not lower");

    assert_eq!(error.entity(), Some(EntityId(110)), "names the bad loop");
    assert!(
        !error.is_unsupported(),
        "this is corrupt input, not an unimplemented family: {error}"
    );
}

/// A face with a hole keeps both bounds and marks which one is outer.
///
/// `IfcFaceOuterBound` and `IfcFaceBound` are different types carrying the
/// same attributes; only the type name says which is the hole. Treating every
/// bound as outer loses the hole entirely.
#[test]
fn an_inner_bound_is_kept_and_marked_as_not_outer() {
    let mut model = Model::new();
    let shell = tetra_shell(&mut model, 100, [0.0, 0.0, 0.0], 20.0);

    // A triangular hole inside the first face, as a plain IFCFACEBOUND.
    point(&mut model, 300, [1.0, 1.0, 0.0]);
    point(&mut model, 301, [2.0, 1.0, 0.0]);
    point(&mut model, 302, [1.0, 2.0, 0.0]);
    model.insert(
        EntityId(310),
        Entity::new("IFCPOLYLOOP", vec![refs(&[300, 301, 302])]),
    );
    model.insert(
        EntityId(311),
        Entity::new(
            "IFCFACEBOUND",
            vec![Value::Ref(EntityId(310)), Value::Bool(true)],
        ),
    );
    // Face #112 gains the inner bound alongside its outer bound #111.
    model.insert(
        EntityId(112),
        Entity::new("IFCFACE", vec![refs(&[111, 311])]),
    );
    model.insert(
        EntityId(900),
        Entity::new("IFCFACETEDBREP", vec![Value::Ref(EntityId(shell))]),
    );

    let scale = units::resolve(&model);
    let mut session = LoweringSession::new(&model, &scale, Tolerance::building_scale());
    let node = lower_faceted_brep_node(&mut session, EntityId(900), Transform::identity())
        .expect("lowers");
    let lowered = session.finish(node).expect("finishes");
    let brep = brep_of(&lowered.graph, lowered.root);

    let holed = brep
        .faces()
        .iter()
        .find(|face| face.bounds.len() == 2)
        .expect("one face carries an inner bound");
    assert_eq!(
        holed.bounds.iter().filter(|b| b.outer).count(),
        1,
        "exactly one bound is the outer one"
    );
    assert_eq!(
        holed.bounds.iter().filter(|b| !b.outer).count(),
        1,
        "the hole is preserved and marked inner"
    );
}

/// The dispatcher routes faceted breps instead of reporting them unsupported.
///
/// Implementing a family is only half the work: it must be reachable through
/// the one total entry point that nested families (booleans, mapped items)
/// call. Without the dispatch arm a brep inside a mapped item silently stays
/// unsupported even though the lowerer exists.
#[test]
fn the_dispatcher_routes_faceted_breps() {
    use ifc_geometry::lower::dispatch::{IMPLEMENTED, PLANNED};
    use ifc_geometry::lower::lower_representation_item;

    assert!(
        IMPLEMENTED.contains(&"IFCFACETEDBREP"),
        "the census table must list the family as implemented"
    );
    assert!(
        !PLANNED.iter().any(|(name, _)| *name == "IFCFACETEDBREP"),
        "a family cannot be both implemented and planned"
    );

    let model = model_of("ifclite-geometry/issue_1985_scaled_kinds.ifc");
    let scale = units::resolve(&model);
    let id = *model.ids_of_type("IFCFACETEDBREP").first().expect("brep");

    let mut session = LoweringSession::new(&model, &scale, Tolerance::building_scale());
    let node = lower_representation_item(&mut session, id, Transform::identity())
        .expect("the dispatcher must route a faceted brep");
    let lowered = session.finish(node).expect("finishes");
    assert!(
        matches!(
            lowered.graph.get(lowered.root).expect("root"),
            GeometryNode::BRep(_)
        ),
        "dispatch must yield the brep node, not a substitute"
    );
}
