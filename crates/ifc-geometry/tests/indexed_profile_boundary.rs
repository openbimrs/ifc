//! `IfcIndexedPolyCurve` profile boundaries (#335).
//!
//! The fixtures (`tools/gen_lowering_fixtures.py`,
//! `indexed_profile_boundaries.ifc` in IFC4 and
//! `indexed_profile_boundaries_ifc4x3.ifc` in IFC4X3_ADD2, whose point lists
//! carry a `TagList`) put each indexed boundary next to the `IfcPolyline` or
//! `IfcCompositeCurve` that states the same outline, by `ProfileName`:
//!
//! - `INDEXED_D` / `COMPOSITE_D`: the square `[-1, 1]^2` whose right edge is
//!   the half circle through `(1,-1)`, `(2,0)`, `(1,1)`; area `4 + pi/2`.
//!   The composite trims an `IfcCircle` about `(1, 0)` at Cartesian points.
//! - `INDEXED_D_CLOCKWISE` / `COMPOSITE_D_CLOCKWISE`: the same, traversed
//!   clockwise; the composite's arc has `SenseAgreement` FALSE.
//! - `INDEXED_POLYLINE` / `POLYLINE`: a 2 x 1 rectangle, the indexed curve
//!   without `Segments`, closed by repeating its first point.
//! - `INDEXED_WITH_VOIDS`: an `IfcArbitraryProfileDefWithVoids`, the 4 x 4
//!   square as one `IfcLineIndex` and a unit circle of two `IfcArcIndex`
//!   segments as its void; area `16 - pi`.
//!
//! The refusals edit one record of the IFC4 fixture in memory: a committed
//! fixture must lower (`tests/lower_dispatch_corpus.rs`).

#![cfg(feature = "lowering")]

use std::path::PathBuf;

use axiolid_curve::Curve2;
use axiolid_profile::{ContourProfile, Profile};
use ifc_geometry::lower::lower_profile;
use ifc_geometry::{units, GeometryError, GeometryResult};
use ifc_model::{Codec, EntityId, Model, Value};
use ifc_step::StepCodec;

const IFC4: &str = "indexed_profile_boundaries.ifc";
const IFC4X3: &str = "indexed_profile_boundaries_ifc4x3.ifc";

fn text(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../test/fixtures/synthetic-lowering")
        .join(name);
    std::fs::read_to_string(path).expect("fixture reads")
}

fn parse(text: &str) -> Model {
    StepCodec
        .read_bytes(text.as_bytes())
        .expect("fixture parses")
}

/// The IFC4 fixture with `record` replaced by `replacement`.
fn edited(record: &str, replacement: &str) -> Model {
    let original = text(IFC4);
    assert_eq!(original.matches(record).count(), 1, "{record} is unique");
    parse(&original.replace(record, replacement))
}

/// The profile whose `ProfileName` is `name`.
fn profile_id(model: &Model, name: &str) -> EntityId {
    let found: Vec<EntityId> = ["IFCARBITRARYCLOSEDPROFILEDEF", "IFCARBITRARYPROFILEDEFWITHVOIDS"]
        .into_iter()
        .flat_map(|kind| model.of_type(kind))
        .filter(|(_, entity)| {
            matches!(entity.attributes.get(1), Some(Value::Text(text)) if &**text == name)
        })
        .map(|(id, _)| id)
        .collect();
    assert_eq!(found.len(), 1, "exactly one profile named {name}");
    found[0]
}

fn lowered(model: &Model, name: &str) -> GeometryResult<Profile> {
    lower_profile(model, profile_id(model, name), &units::resolve(model))
}

fn contour(model: &Model, name: &str) -> ContourProfile {
    match lowered(model, name) {
        Ok(Profile::Contour(contour)) => contour,
        other => panic!("{name}: expected a contour profile, got {other:?}"),
    }
}

/// The issue's acceptance test: an indexed boundary with an arc segment
/// lowers to the same neutral profile as its `IfcCompositeCurve`
/// equivalent, in both releases and both directions.
#[test]
fn an_indexed_arc_boundary_lowers_like_its_composite_equivalent() {
    for fixture in [IFC4, IFC4X3] {
        let model = parse(&text(fixture));
        for (indexed, composite) in [
            ("INDEXED_D", "COMPOSITE_D"),
            ("INDEXED_D_CLOCKWISE", "COMPOSITE_D_CLOCKWISE"),
        ] {
            let lowered = contour(&model, indexed);
            assert_eq!(
                lowered,
                contour(&model, composite),
                "{fixture}: {indexed} differs from {composite}"
            );
            let arcs: Vec<_> = lowered
                .outer
                .segments
                .iter()
                .filter(|segment| matches!(segment.curve, Curve2::Circle(_)))
                .collect();
            assert_eq!(arcs.len(), 1, "{fixture}: {indexed} keeps one exact arc");
            let Curve2::Circle(circle) = arcs[0].curve else {
                unreachable!()
            };
            assert_eq!(circle.radius, 1.0);
            assert_eq!(circle.frame.origin.to_array(), [1.0, 0.0]);
            assert!(
                (arcs[0].domain.end - arcs[0].domain.start - std::f64::consts::PI).abs() < 1e-12
            );
            assert_eq!(
                arcs[0].same_sense,
                indexed == "INDEXED_D",
                "{fixture}: {indexed} runs the arc the authored way"
            );
        }
    }
}

/// Without `Segments` the points are one polyline: the same contour as the
/// `IfcPolyline` through them.
#[test]
fn an_indexed_curve_without_segments_lowers_like_its_polyline() {
    for fixture in [IFC4, IFC4X3] {
        let model = parse(&text(fixture));
        let indexed = contour(&model, "INDEXED_POLYLINE");
        assert_eq!(indexed, contour(&model, "POLYLINE"), "{fixture}");
        assert_eq!(indexed.outer.len(), 4);
        assert!(indexed.holes.is_empty());
    }
}

/// `InnerCurves` lower too: a void of two arcs, each an exact half circle.
#[test]
fn indexed_inner_curves_lower_as_exact_voids() {
    for fixture in [IFC4, IFC4X3] {
        let model = parse(&text(fixture));
        let profile = contour(&model, "INDEXED_WITH_VOIDS");
        assert_eq!(
            profile.outer.len(),
            4,
            "{fixture}: one IfcLineIndex, four edges"
        );
        assert_eq!(profile.holes.len(), 1);
        let hole = &profile.holes[0];
        assert_eq!(hole.len(), 2, "{fixture}: two arcs");
        for segment in &hole.segments {
            let Curve2::Circle(circle) = segment.curve else {
                panic!("{fixture}: the void is two arcs, got {:?}", segment.curve)
            };
            assert!((circle.radius - 1.0).abs() < 1e-12);
            assert!(circle.frame.origin.length() < 1e-12);
            assert!(
                (segment.domain.end - segment.domain.start - std::f64::consts::PI).abs() < 1e-12
            );
            assert!(segment.same_sense, "anticlockwise");
        }
    }
}

/// The refusal's entity and detail.
fn refusal(model: &Model, name: &str) -> (EntityId, String) {
    match lowered(model, name) {
        Err(GeometryError::Degenerate {
            entity,
            type_name,
            detail,
        }) => {
            assert!(
                type_name.starts_with("IFCINDEXEDPOLYCURVE")
                    || type_name == "IFCCARTESIANPOINTLIST3D",
                "{type_name}"
            );
            (entity, detail)
        }
        other => panic!("{name}: expected a Degenerate refusal, got {other:?}"),
    }
}

const D_CURVE: &str =
    "#15=IFCINDEXEDPOLYCURVE(#14,(IFCLINEINDEX((1,2)),IFCARCINDEX((2,3,4)),IFCLINEINDEX((4,5,1))),.F.);";
const D_POINTS: &str =
    "#14=IFCCARTESIANPOINTLIST2D(((-1.,-1.),(1.,-1.),(2.,0.),(1.,1.),(-1.,1.)));";

#[test]
fn an_open_indexed_curve_is_refused_not_closed() {
    let model = edited(D_CURVE, &D_CURVE.replace("(4,5,1)", "(4,5)"));
    let (entity, detail) = refusal(&model, "INDEXED_D");
    assert_eq!(entity, EntityId(15));
    assert!(detail.contains("open"), "{detail}");

    // Without Segments: the last point is not the first.
    let model = edited(
        "#56=IFCCARTESIANPOINTLIST2D(((0.,0.),(2.,0.),(2.,1.),(0.,1.),(0.,0.)));",
        "#56=IFCCARTESIANPOINTLIST2D(((0.,0.),(2.,0.),(2.,1.),(0.,1.)));",
    );
    let (entity, detail) = refusal(&model, "INDEXED_POLYLINE");
    assert_eq!(entity, EntityId(57));
    assert!(detail.contains("open"), "{detail}");
}

/// The straight edges of a lowered outer contour, as `[origin, end]` pairs;
/// panics on a curved edge.
fn straight_edges(profile: &ContourProfile) -> Vec<[[f64; 2]; 2]> {
    profile
        .outer
        .segments
        .iter()
        .map(|segment| match &segment.curve {
            Curve2::Line(line) => [
                line.origin.to_array(),
                (line.origin + line.direction).to_array(),
            ],
            other => panic!("expected only straight edges, got {other:?}"),
        })
        .collect()
}

/// "In case that this informal proposition is not maintained, the arc
/// segment shall be treated as a polyline segment": a collinear arc lowers
/// as the straight path start -> mid -> end.
#[test]
fn a_collinear_arc_is_treated_as_a_polyline_segment() {
    // The middle point between the others: one edge, (1,-1) -> (1,1).
    let model = edited(D_POINTS, &D_POINTS.replace("(2.,0.)", "(1.,0.)"));
    assert_eq!(
        straight_edges(&contour(&model, "INDEXED_D")),
        [
            [[-1.0, -1.0], [1.0, -1.0]],
            [[1.0, -1.0], [1.0, 1.0]],
            [[1.0, 1.0], [-1.0, 1.0]],
            [[-1.0, 1.0], [-1.0, -1.0]],
        ]
    );

    // The middle point beyond the end: two edges, out and back.
    let model = edited(D_POINTS, &D_POINTS.replace("(2.,0.)", "(1.,2.)"));
    assert_eq!(
        straight_edges(&contour(&model, "INDEXED_D"))[1..3],
        [[[1.0, -1.0], [1.0, 2.0]], [[1.0, 2.0], [1.0, 1.0]]]
    );
}

/// Collinearity is judged "after taking the Precision factor into
/// account": the context's 1e-5 m makes a 1e-6 m bulge straight, and a
/// declared 1e-7 m keeps it an arc.
#[test]
fn collinearity_uses_the_model_precision() {
    let bulge = D_POINTS.replace("(2.,0.)", "(1.000001,0.)");
    let model = edited(D_POINTS, &bulge);
    assert!(straight_edges(&contour(&model, "INDEXED_D")).len() == 4);

    let context = "#3=IFCGEOMETRICREPRESENTATIONCONTEXT($,'Model',3,1.0000000000000001E-05,#2,$);";
    let original = text(IFC4);
    assert_eq!(original.matches(context).count(), 1);
    let fine = parse(&original.replace(D_POINTS, &bulge).replace(
        context,
        &context.replace("1.0000000000000001E-05", "1.E-07"),
    ));
    let arcs = contour(&fine, "INDEXED_D")
        .outer
        .segments
        .iter()
        .filter(|segment| matches!(segment.curve, Curve2::Circle(_)))
        .count();
    assert_eq!(arcs, 1, "a 1e-6 m bulge is an arc under a 1e-7 m precision");
}

/// Closure without `Segments` is judged within the model's precision too.
#[test]
fn closure_without_segments_uses_the_model_precision() {
    let list = "#56=IFCCARTESIANPOINTLIST2D(((0.,0.),(2.,0.),(2.,1.),(0.,1.),(0.,0.)));";
    let near = list.replace("(0.,0.)));", "(0.000001,0.)));");
    let model = edited(list, &near);
    assert_eq!(
        contour(&model, "INDEXED_POLYLINE").outer.len(),
        4,
        "1e-6 m closes"
    );

    let model = edited(list, &list.replace("(0.,0.)));", "(0.0001,0.)));"));
    let (entity, detail) = refusal(&model, "INDEXED_POLYLINE");
    assert_eq!(entity, EntityId(57));
    assert!(detail.contains("open"), "1e-4 m is open: {detail}");
}

#[test]
fn an_arc_with_coincident_points_is_refused() {
    let model = edited(D_CURVE, &D_CURVE.replace("(2,3,4)", "(2,2,4)"));
    let (entity, detail) = refusal(&model, "INDEXED_D");
    assert_eq!(entity, EntityId(15));
    assert!(detail.contains("coincident"), "{detail}");
}

#[test]
fn a_self_intersecting_boundary_is_refused() {
    let model = edited(D_CURVE, &D_CURVE.replace(".F.);", ".T.);"));
    let (entity, detail) = refusal(&model, "INDEXED_D");
    assert_eq!(entity, EntityId(15));
    assert!(detail.contains("SelfIntersect"), "{detail}");
}

#[test]
fn non_consecutive_segments_are_refused() {
    let model = edited(D_CURVE, &D_CURVE.replace("(4,5,1)", "(5,1)"));
    let (entity, detail) = refusal(&model, "INDEXED_D");
    assert_eq!(entity, EntityId(15));
    assert!(detail.contains("Consecutive"), "{detail}");
}

#[test]
fn a_3d_point_list_is_refused() {
    let model = edited(
        D_POINTS,
        "#14=IFCCARTESIANPOINTLIST3D(((-1.,-1.,0.),(1.,-1.,0.),(2.,0.,0.),(1.,1.,0.),(-1.,1.,0.)));",
    );
    let (entity, detail) = refusal(&model, "INDEXED_D");
    assert_eq!(entity, EntityId(14));
    assert!(detail.contains("3D"), "{detail}");
}

/// An inner curve refuses by its own id, so the void that failed is named.
#[test]
fn an_open_inner_curve_is_refused_by_its_own_id() {
    let curve = "#69=IFCINDEXEDPOLYCURVE(#68,(IFCARCINDEX((1,2,3)),IFCARCINDEX((3,4,1))),.F.);";
    let model = edited(curve, &curve.replace("(3,4,1)", "(3,4,2)"));
    let (entity, detail) = refusal(&model, "INDEXED_WITH_VOIDS");
    assert_eq!(entity, EntityId(69));
    assert!(detail.contains("open"), "{detail}");
}

#[cfg(feature = "compile-reference-backend")]
mod compiled {
    //! Each profile is extruded by 1 m, so the volume in cubic metres is
    //! its area. Arcs are flattened by the kernel under
    //! `Tolerance::MILLIMETRE`; a lost sliver is far below 0.01 m3, every
    //! plausible mistake (arc on the wrong side, the long way round) far
    //! above it.

    use super::*;
    use axiolid_core::Tolerance;
    use axiolid_mesh::TriMesh;
    use ifc_geometry::compile::compile_product_mesh;
    use std::f64::consts::PI;

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

    fn volume(mesh: &TriMesh) -> f64 {
        let base = mesh.positions[0];
        mesh.indices
            .chunks_exact(3)
            .map(|t| {
                let [a, b, c] = [t[0], t[1], t[2]].map(|i| mesh.positions[i as usize] - base);
                a.dot(b.cross(c)) / 6.0
            })
            .sum()
    }

    #[test]
    fn indexed_profiles_compile_to_their_analytic_areas() {
        for fixture in [IFC4, IFC4X3] {
            let model = parse(&text(fixture));
            for (name, area) in [
                ("INDEXED_D", 4.0 + PI / 2.0),
                ("INDEXED_D_CLOCKWISE", 4.0 + PI / 2.0),
                ("INDEXED_POLYLINE", 2.0),
                ("INDEXED_WITH_VOIDS", 16.0 - PI),
            ] {
                let mesh =
                    compile_product_mesh(&model, product(&model, name), Tolerance::MILLIMETRE)
                        .unwrap_or_else(|error| panic!("{fixture} {name} compiles: {error}"))
                        .expect("the product has a body");
                let volume = volume(&mesh);
                assert!(
                    (volume - area).abs() < 0.01,
                    "{fixture} {name}: volume {volume}, expected {area}"
                );
            }
        }
    }
}
