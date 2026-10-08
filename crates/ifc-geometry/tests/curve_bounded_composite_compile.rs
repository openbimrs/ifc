//! `IfcCurveBoundedPlane` boundaries made of polyline segments compile (#336).
//!
//! The boundary of a curve-bounded plane lowers as written: an
//! `IfcCompositeCurve` becomes a `CurveRelation::Composite` whose segments
//! keep their `SameSense`. Up to `axiolid-mesh-compile` 0.3.12 the reference
//! compiler refused such a boundary ("is not a curve node"); 0.3.13 resolves
//! `Composite` and `Trimmed` relations to points before flattening, honouring
//! each segment's sense (axiolid/kernel#255). Nothing is merged on the IFC
//! side, so the segment structure stays in the neutral graph.
//!
//! The fixture is `test/fixtures/synthetic-lowering/curve_bounded_plane_composite.ifc`
//! (`tools/gen_lowering_fixtures.py`): every boundary is straight, so the
//! compiled area is exact.

#![cfg(feature = "compile-reference-backend")]

use std::path::PathBuf;

use axiolid_core::{Tolerance, Vec3};
use axiolid_mesh::TriMesh;
use axiolid_model::{CurveRelation, GeometryNode, SurfaceRelation};
use ifc_geometry::compile::compile_product_mesh;
use ifc_geometry::lower::{lower_representation_item, LoweringSession};
use ifc_geometry::transform::Transform;
use ifc_geometry::units;
use ifc_model::{Codec, EntityId, Model, Value};

fn fixture() -> Model {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../test/fixtures/synthetic-lowering/curve_bounded_plane_composite.ifc");
    ifc_step::StepCodec
        .read_path(&path)
        .expect("fixture parses")
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

fn mesh(model: &Model, name: &str) -> TriMesh {
    compile_product_mesh(model, product(model, name), Tolerance::MILLIMETRE)
        .unwrap_or_else(|error| panic!("{name} compiles: {error}"))
        .expect("the product has a body")
}

/// The mesh's area, as the length of its summed area vector: the plane is
/// flat, so every triangle of a correctly oriented mesh adds to it.
fn area(mesh: &TriMesh) -> f64 {
    mesh.indices
        .chunks_exact(3)
        .map(|t| {
            let [a, b, c] = [t[0], t[1], t[2]].map(|i| mesh.positions[i as usize]);
            (b - a).cross(c - a) * 0.5
        })
        .fold(Vec3::ZERO, |sum, v| sum + v)
        .length()
}

/// Every triangle's own area, summed: equal to [`area`] only when no
/// triangle folds back over another.
fn unsigned_area(mesh: &TriMesh) -> f64 {
    mesh.indices
        .chunks_exact(3)
        .map(|t| {
            let [a, b, c] = [t[0], t[1], t[2]].map(|i| mesh.positions[i as usize]);
            (b - a).cross(c - a).length() * 0.5
        })
        .sum()
}

/// Three polyline segments, the middle one authored backwards with
/// `SameSense` FALSE, around a hole of two polyline segments: 4 x 3 less
/// 1 x 1.
#[test]
fn a_multi_segment_polyline_composite_with_a_hole_compiles_exactly() {
    let model = fixture();
    let mesh = mesh(&model, "SEGMENTS_WITH_HOLE");
    assert!((area(&mesh) - 11.0).abs() < 1e-9, "area {}", area(&mesh));
    assert!((unsigned_area(&mesh) - 11.0).abs() < 1e-9);
    for p in &mesh.positions {
        assert!(p.z.abs() < 1e-12, "{p:?} is off the plane z = 0");
        assert!((-1e-9..=4.0 + 1e-9).contains(&p.x), "{p:?}");
        assert!((-1e-9..=3.0 + 1e-9).contains(&p.y), "{p:?}");
        let in_hole = p.x > 1.0 + 1e-9 && p.x < 2.0 - 1e-9 && p.y > 1.0 + 1e-9 && p.y < 2.0 - 1e-9;
        assert!(!in_hole, "{p:?} lies inside the hole");
    }
}

/// axiolid/kernel#255's own acceptance case: one segment with `SameSense`
/// FALSE wrapping a clockwise closed polyline compiles like the polyline
/// read anticlockwise.
#[test]
fn a_one_segment_reversed_composite_compiles_like_its_polyline() {
    let model = fixture();
    let mesh = mesh(&model, "ONE_REVERSED_SEGMENT");
    assert!((area(&mesh) - 12.0).abs() < 1e-9, "area {}", area(&mesh));
    assert!((unsigned_area(&mesh) - 12.0).abs() < 1e-9);
    assert!(mesh.positions.iter().all(|p| (p.z - 5.0).abs() < 1e-12));
}

/// The boundaries reach the kernel as composites, segment for segment, with
/// each segment's sense: nothing is merged into one polyline on this side.
#[test]
fn the_boundaries_keep_their_segments_and_senses() {
    let model = fixture();
    let scale = units::resolve(&model);
    let plane = model
        .of_type("IFCCURVEBOUNDEDPLANE")
        .map(|(id, _)| id)
        .min()
        .expect("a curve-bounded plane");
    let mut session = LoweringSession::new(&model, &scale);
    let root =
        lower_representation_item(&mut session, plane, Transform::identity()).expect("lowers");
    let lowered = session.finish(root).expect("session finishes");
    let graph = &lowered.graph;
    let Some(GeometryNode::SurfaceRelation(SurfaceRelation::CurveBounded { boundaries, .. })) =
        graph.get(lowered.root)
    else {
        panic!("expected a curve-bounded surface");
    };
    let senses = |boundary| match graph.get(boundary) {
        Some(GeometryNode::CurveRelation(CurveRelation::Composite { segments })) => {
            segments.iter().map(|s| s.same_sense).collect::<Vec<_>>()
        }
        other => panic!("expected a composite boundary, got {other:?}"),
    };
    assert_eq!(senses(boundaries[0]), [true, false, true]);
    assert_eq!(senses(boundaries[1]), [true, true]);
}
