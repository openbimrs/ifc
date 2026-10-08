//! `IfcGridPlacement` resolution (#362) and placements relative to a grid
//! placement (#363).
//!
//! The fixtures (`tools/gen_lowering_fixtures.py`, `grid_placement.ifc` in
//! IFC4 and `grid_placement_ifc4x3.ifc` in IFC4X3_ADD2) state one grid: the
//! storey at (0, 0, 3), the grid at (10, 5, 0) relative to it with its x
//! along (0.6, 0.8, 0). Expected values are computed here from the
//! generator's description, in the grid's frame, then mapped by
//! `grid(p) = (10 + 0.6 x - 0.8 y, 5 + 0.8 x + 0.6 y, 3 + z)`.

#![cfg(feature = "lowering")]

use ifc_geometry::lower::{lower_product_items, LoweringSession};
use ifc_geometry::{product_world_transform, units, GeometryError, Transform};
use ifc_model::{Codec, EntityId, Model, Value};
use std::path::PathBuf;

const TOL: f64 = 1e-9;

const FIXTURES: [&str; 2] = ["grid_placement.ifc", "grid_placement_ifc4x3.ifc"];

fn fixture(name: &str) -> Model {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../test/fixtures/synthetic-lowering")
        .join(name);
    ifc_step::StepCodec
        .read_path(&path)
        .expect("fixture parses")
}

fn ifc4() -> Model {
    fixture(FIXTURES[0])
}

/// The proxy named `name`.
fn product(model: &Model, name: &str) -> EntityId {
    let found: Vec<EntityId> = model
        .of_type("IFCBUILDINGELEMENTPROXY")
        .filter(|(_, entity)| {
            matches!(entity.attributes.get(2), Some(Value::Text(text)) if &**text == name)
        })
        .map(|(id, _)| id)
        .collect();
    assert_eq!(found.len(), 1, "exactly one proxy named {name}");
    found[0]
}

/// The `ObjectPlacement` of `id`.
fn placement_of(model: &Model, id: EntityId) -> EntityId {
    model
        .get(id)
        .and_then(|entity| entity.attributes.get(5))
        .and_then(Value::as_ref_id)
        .expect("placed")
}

/// The grid axis tagged `tag`.
fn axis(model: &Model, tag: &str) -> EntityId {
    model
        .of_type("IFCGRIDAXIS")
        .find(|(_, entity)| {
            matches!(entity.attributes.first(), Some(Value::Text(text)) if &**text == tag)
        })
        .map(|(id, _)| id)
        .expect("an axis with that tag")
}

/// The `PlacementLocation` of a product's grid placement.
fn location_of(model: &Model, name: &str) -> EntityId {
    let placement = placement_of(model, product(model, name));
    let entity = model.get(placement).expect("exists");
    let slot = entity.attributes.len() - 2;
    entity.attributes[slot].as_ref_id().expect("a location")
}

/// Replace attribute `slot` of `id`.
fn set(model: &mut Model, id: EntityId, slot: usize, value: Value) {
    let mut entity = model.get(id).expect("exists").clone();
    entity.attributes[slot] = value;
    model.insert(id, entity);
}

fn grid(p: [f64; 3]) -> [f64; 3] {
    [
        10.0 + 0.6 * p[0] - 0.8 * p[1],
        5.0 + 0.8 * p[0] + 0.6 * p[1],
        3.0 + p[2],
    ]
}

fn grid_dir(v: [f64; 2]) -> [f64; 3] {
    [0.6 * v[0] - 0.8 * v[1], 0.8 * v[0] + 0.6 * v[1], 0.0]
}

fn assert_vec(actual: [f64; 3], expected: [f64; 3], what: &str) {
    for axis in 0..3 {
        assert!(
            (actual[axis] - expected[axis]).abs() < TOL,
            "{what}: got {actual:?}, expected {expected:?}"
        );
    }
}

/// A placement frame with `origin` (grid frame) and x along `x` (grid XY).
fn assert_grid_frame(world: &Transform, origin: [f64; 3], x: [f64; 2], what: &str) {
    let length = x[0].hypot(x[1]);
    let x = [x[0] / length, x[1] / length];
    assert_vec(world.origin, grid(origin), &format!("{what}: origin"));
    assert_vec(world.basis[0], grid_dir(x), &format!("{what}: x"));
    assert_vec(
        world.basis[1],
        grid_dir([-x[1], x[0]]),
        &format!("{what}: y"),
    );
    assert_vec(world.basis[2], [0.0, 0.0, 1.0], &format!("{what}: z"));
}

fn world(model: &Model, name: &str) -> Result<Transform, GeometryError> {
    product_world_transform(model, &units::resolve(model), product(model, name))
}

fn lowered_centre(model: &Model, name: &str) -> Result<[f64; 3], GeometryError> {
    let scale = units::resolve(model);
    let mut session = LoweringSession::new(model, &scale);
    let root = lower_product_items(&mut session, product(model, name))?.expect("a Body");
    let lowered = session.finish(root)?;
    let graph = &lowered.graph;
    let mut sum = [0.0f64; 3];
    let mut count = 0usize;
    let mut stack = vec![(lowered.root, axiolid_core::Transform3::IDENTITY)];
    while let Some((id, frame)) = stack.pop() {
        match graph.get(id).expect("node exists") {
            axiolid_model::GeometryNode::Instance(instance) => {
                let composed = frame * instance.transform;
                let t = composed.translation;
                sum = [sum[0] + t.x, sum[1] + t.y, sum[2] + t.z];
                count += 1;
                stack.push((instance.source, composed));
            }
            axiolid_model::GeometryNode::Collection(members) => {
                stack.extend(members.iter().map(|member| (*member, frame)));
            }
            _ => {}
        }
    }
    assert!(count > 0, "the block is placed by an Instance");
    Ok(sum.map(|value| value / count as f64))
}

// ---- #362: straight axes in closed form -------------------------------------

#[test]
fn grid_placement_an_intersection_with_offsets_lowers_to_the_expected_position() {
    for name in FIXTURES {
        let model = fixture(name);
        // A runs +y, so its left is -x: x = -0.5. Axis 1 runs +x, its left
        // is +y: y = -0.25. The third offset is along the grid's z.
        let world = world(&model, "A1_OFFSET").expect(name);
        assert_grid_frame(&world, [-0.5, -0.25, 1.0], [0.0, 1.0], name);
        let centre = lowered_centre(&model, "A1_OFFSET").expect("lowers");
        assert_vec(
            centre,
            grid([-0.5, -0.25, 1.5]),
            "block centre, half a metre up",
        );
    }
}

#[test]
fn grid_placement_a_reference_intersection_sets_the_rotation() {
    for name in FIXTURES {
        let model = fixture(name);
        // From A/1 (0, 0) towards B/2 (6, 8).
        let world = world(&model, "A1_TOWARDS_B2").expect(name);
        assert_grid_frame(&world, [0.0, 0.0, 0.0], [0.6, 0.8], name);
    }
}

#[test]
fn grid_placement_a_reference_direction_takes_its_x_and_y_ratios() {
    for name in FIXTURES {
        let model = fixture(name);
        // IfcDirection (1, 1, 5): "only the ratios for x and y are taken
        // into account". B/2 is (6, 8): the trimmed line of axis 2 is y = 8.
        let world = world(&model, "B2_DIRECTION").expect(name);
        assert_grid_frame(&world, [6.0, 8.0, 0.0], [1.0, 1.0], name);
    }
}

#[test]
fn grid_placement_same_sense_false_reverts_the_axis_and_its_offset() {
    for name in FIXTURES {
        let model = fixture(name);
        // B runs -y once reverted: its left is +x, so x = 6.5, and the
        // placement's x follows B, (0, -1).
        let world = world(&model, "B1_SAMESENSE").expect(name);
        assert_grid_frame(&world, [6.5, 0.0, 0.0], [0.0, -1.0], name);
    }
}

#[test]
fn grid_placement_ifc4_and_ifc4x3_layouts_agree() {
    let (a, b) = (fixture(FIXTURES[0]), fixture(FIXTURES[1]));
    for name in [
        "A1_OFFSET",
        "A1_TOWARDS_B2",
        "B2_DIRECTION",
        "B1_SAMESENSE",
        "BRACKET",
    ] {
        let (x, y) = (
            world(&a, name).expect("ifc4"),
            world(&b, name).expect("ifc4x3"),
        );
        assert_vec(x.origin, y.origin, name);
        for i in 0..3 {
            assert_vec(x.basis[i], y.basis[i], name);
        }
    }
}

#[test]
fn grid_placement_curved_axes_without_an_evaluator_are_refused_by_name() {
    for name in FIXTURES {
        let model = fixture(name);
        let error = world(&model, "R3_CURVED").expect_err("R is an arc");
        let GeometryError::Unsupported {
            type_name, detail, ..
        } = &error
        else {
            panic!("expected Unsupported, got {error:?}");
        };
        assert_eq!(type_name, "IFCTRIMMEDCURVE");
        assert!(detail.contains("CurveEvaluator"), "{detail}");
        let error = lowered_centre(&model, "R3_CURVED").expect_err("lowering refuses too");
        assert!(error.is_unsupported(), "{error}");
    }
}

// ---- #362: distinct refusals --------------------------------------------------

#[test]
fn grid_placement_parallel_axes_are_refused() {
    let mut model = ifc4();
    let location = location_of(&model, "A1_OFFSET");
    let (a, b) = (axis(&model, "A"), axis(&model, "B"));
    set(
        &mut model,
        location,
        0,
        Value::List(vec![Value::Ref(a), Value::Ref(b)]),
    );
    let error = world(&model, "A1_OFFSET").expect_err("A and B are parallel");
    assert!(
        matches!(error, GeometryError::GridAxesParallel { intersection, axes }
            if intersection == location && axes == [a, b]),
        "{error:?}"
    );
}

#[test]
fn grid_placement_an_axis_in_no_grid_is_refused() {
    let mut model = ifc4();
    let location = location_of(&model, "A1_OFFSET");
    let a = axis(&model, "A");
    let mut orphan = model.get(a).expect("exists").clone();
    orphan.attributes[0] = Value::Text("ORPHAN".into());
    let orphan = model.push(orphan);
    let one = axis(&model, "1");
    set(
        &mut model,
        location,
        0,
        Value::List(vec![Value::Ref(orphan), Value::Ref(one)]),
    );
    let error = world(&model, "A1_OFFSET").expect_err("no grid lists it");
    assert!(
        matches!(error, GeometryError::GridAxisWithoutGrid { axis } if axis == orphan),
        "{error:?}"
    );
}

#[test]
fn grid_placement_axes_of_two_grids_are_refused() {
    let mut model = ifc4();
    let location = location_of(&model, "A1_OFFSET");
    let (first_grid, entity) = model.of_type("IFCGRID").next().expect("a grid");
    let mut second = entity.clone();
    let a = axis(&model, "A");
    let mut moved = model.get(a).expect("exists").clone();
    moved.attributes[0] = Value::Text("A2".into());
    let moved = model.push(moved);
    second.attributes[0] = Value::Text("0000000000000000000002".into());
    second.attributes[7] = Value::List(vec![Value::Ref(moved)]);
    second.attributes[8] = Value::List(vec![]);
    let second = model.push(second);
    let one = axis(&model, "1");
    set(
        &mut model,
        location,
        0,
        Value::List(vec![Value::Ref(moved), Value::Ref(one)]),
    );
    let error = world(&model, "A1_OFFSET").expect_err("two grids");
    assert!(
        matches!(error, GeometryError::GridAxesInDifferentGrids { grids, .. }
            if grids == [second, first_grid]),
        "{error:?}"
    );
}

#[test]
fn grid_placement_a_missing_axis_is_refused() {
    let mut model = ifc4();
    let location = location_of(&model, "A1_OFFSET");
    let one = axis(&model, "1");
    set(
        &mut model,
        location,
        0,
        Value::List(vec![Value::Ref(EntityId(999_999)), Value::Ref(one)]),
    );
    let error = world(&model, "A1_OFFSET").expect_err("dangling");
    assert!(
        matches!(error, GeometryError::MissingEntity { missing, .. } if missing == EntityId(999_999)),
        "{error:?}"
    );
}

#[test]
fn grid_placement_a_grid_without_placement_is_refused() {
    let mut model = ifc4();
    let (grid, _) = model.of_type("IFCGRID").next().expect("a grid");
    set(&mut model, grid, 5, Value::Null);
    let error = world(&model, "A1_OFFSET").expect_err("no grid frame");
    assert!(
        matches!(error, GeometryError::MissingAttribute { entity, attribute: "ObjectPlacement", .. }
            if entity == grid),
        "{error:?}"
    );
}

#[test]
fn grid_placement_ifc4x3_a_conflicting_placement_rel_to_is_refused() {
    let mut model = fixture(FIXTURES[1]);
    let placement = placement_of(&model, product(&model, "A1_OFFSET"));
    let (storey, _) = model.of_type("IFCBUILDINGSTOREY").next().expect("a storey");
    let storey_placement = placement_of(&model, storey);
    set(&mut model, placement, 0, Value::Ref(storey_placement));
    let error = world(&model, "A1_OFFSET").expect_err("not the grid's frame");
    assert!(
        matches!(error, GeometryError::PlacementRelToConflict { placement: p, stated, .. }
            if p == placement && stated == storey_placement),
        "{error:?}"
    );
}

// ---- #363: placements relative to a grid placement ---------------------------

#[test]
fn grid_placement_a_local_placement_relative_to_a_grid_placement_composes() {
    for name in FIXTURES {
        let model = fixture(name);
        // (1, 0, 2) in A1_OFFSET's frame, whose x is the grid's (0, 1).
        let world = world(&model, "BRACKET").expect(name);
        assert_grid_frame(&world, [-0.5, 0.75, 3.0], [0.0, 1.0], name);
        let centre = lowered_centre(&model, "BRACKET").expect("lowers");
        assert_vec(centre, grid([-0.5, 0.75, 3.5]), "bracket centre");
    }
}

#[test]
fn grid_placement_a_mixed_cycle_through_a_grid_placement_is_refused() {
    // The grid placed relative to the bracket that sits on the grid.
    let mut model = ifc4();
    let (grid, _) = model.of_type("IFCGRID").next().expect("a grid");
    let grid_placement = placement_of(&model, grid);
    let bracket = placement_of(&model, product(&model, "BRACKET"));
    set(&mut model, grid_placement, 0, Value::Ref(bracket));
    let error = world(&model, "BRACKET").expect_err("cyclic");
    assert!(
        matches!(error, GeometryError::CyclicChain { .. }),
        "{error:?}"
    );
}

#[test]
fn grid_placement_the_resolver_caches_grid_and_linked_placements() {
    let model = ifc4();
    let mut resolver = ifc_geometry::constraint::local::PlacementResolver::new();
    let bracket = placement_of(&model, product(&model, "BRACKET"));
    let first = resolver.world_transform(&model, bracket).expect("resolves");
    // bracket, grid placement, grid's placement, storey, building, site.
    assert_eq!(resolver.cached(), 6);
    let again = resolver.world_transform(&model, bracket).expect("cached");
    assert_eq!(first, again);
}

// ---- #362: curved axes through the evaluator ---------------------------------

#[cfg(feature = "compile")]
mod curved {
    use super::*;
    use axiolid_evaluate::ReferenceCurveEvaluator;
    use ifc_geometry::{product_world_transform_with_evaluator, CachedPositionPolicy};

    #[test]
    fn grid_placement_curved_axes_resolve_through_the_evaluator() {
        // R runs anticlockwise round radius 10; its left is inward, so the
        // 0.5 offset curve has radius 9.5, meeting axis 3 (y = 5) at
        // (sqrt(65.25), 5). The x-axis is R's tangent there.
        let (radius, y) = (9.5f64, 5.0f64);
        let x = (radius * radius - y * y).sqrt();
        let (sin, cos) = (y / radius, x / radius);
        for name in FIXTURES {
            let model = fixture(name);
            let evaluator = ReferenceCurveEvaluator::default();
            let world = product_world_transform_with_evaluator(
                &model,
                &units::resolve(&model),
                product(&model, "R3_CURVED"),
                &evaluator,
                CachedPositionPolicy::Verify,
            )
            .expect("intersected through the evaluator");
            assert_grid_frame(&world, [x, y, 0.0], [-sin, cos], name);

            let scale = units::resolve(&model);
            let mut session = LoweringSession::new(&model, &scale).with_curve_evaluator(&evaluator);
            let root = lower_product_items(&mut session, product(&model, "R3_CURVED"))
                .expect("lowers")
                .expect("a Body");
            session.finish(root).expect("finishes");
        }
    }

    #[test]
    fn grid_placement_curved_axes_that_miss_are_refused() {
        // Offset R inward by 6: radius 4 never reaches y = 5.
        let mut model = ifc4();
        let location = location_of(&model, "R3_CURVED");
        set(
            &mut model,
            location,
            1,
            Value::List(vec![Value::Real(6.0), Value::Real(0.0)]),
        );
        let error = product_world_transform_with_evaluator(
            &model,
            &units::resolve(&model),
            product(&model, "R3_CURVED"),
            &ReferenceCurveEvaluator::default(),
            CachedPositionPolicy::Verify,
        )
        .expect_err("no intersection");
        assert!(
            matches!(error, GeometryError::GridAxesDoNotIntersect { intersection, .. }
                if intersection == location),
            "{error:?}"
        );
    }
}
