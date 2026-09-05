#![cfg(feature = "lowering")]
//! Requires the `lowering` feature: this suite exercises the neutral DAG.
//! Curved, B-spline and bounded surfaces on the synthetic corpus.
//!
//! # Why these fixtures are generated, not sourced
//!
//! We surveyed 909 IFC files across ifc-lite (MPL-2.0), IfcOpenShell and both
//! buildingSMART sample repositories (CC-BY-4.0). None carries a cylindrical,
//! spherical, toroidal, revolved or B-spline surface. The one repository that
//! does -- IfcOpenShell/files -- publishes no licence at all, so its files
//! cannot be redistributed here.
//!
//! `tools/gen_surface_fixtures.py` therefore emits these with ifcopenshell.
//! Generated output is our own data, which removes the licence question
//! entirely, and the generator is committed so the files are reproducible
//! rather than opaque blobs.

use axiolid_model::{CurveRelation, GeometryNode, MasterRepresentation, SurfaceRelation};
use axiolid_reference::surface::Patch;
use axiolid_reference::tessellate::{tessellate_patch, TessellationBudget};
use axiolid_surface::Surface;
use ifc_geometry::lower::{lower_representation_item, lower_surface_node, LoweringSession};
use ifc_geometry::transform::Transform;
use ifc_geometry::units;
use ifc_model::{Codec, EntityId, Model};
use ifc_step::StepCodec;
use std::path::PathBuf;

fn fixture(name: &str) -> Model {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../test/fixtures/synthetic-surfaces")
        .join(name);
    StepCodec
        .read_path(&path)
        .unwrap_or_else(|e| panic!("fixture {} must parse: {e:?}", path.display()))
}

fn lower_one(model: &Model, id: EntityId) -> GeometryNode {
    let scale = units::resolve(model);
    let mut session = LoweringSession::new(model, &scale);
    let node = lower_surface_node(&mut session, id, Transform::identity())
        .unwrap_or_else(|e| panic!("surface {id:?} must lower: {e}"));
    let lowered = session.finish(node).expect("session finishes");
    lowered.graph.get(lowered.root).expect("root node").clone()
}

fn lower_item(model: &Model, id: EntityId) -> ifc_geometry::lower::LoweredGeometry {
    let scale = units::resolve(model);
    let mut session = LoweringSession::new(model, &scale);
    let root = lower_representation_item(&mut session, id, Transform::identity())
        .unwrap_or_else(|e| panic!("item {id:?} must lower: {e}"));
    session.finish(root).expect("session finishes")
}

fn only(model: &Model, kind: &str) -> EntityId {
    let ids = model.ids_of_type(kind);
    assert_eq!(ids.len(), 1, "{kind}: expected exactly one instance");
    ids[0]
}

/// The three curved elementary families lower with millimetre radii resolved.
///
/// The fixture is authored in millimetres, so every radius must come back
/// three orders of magnitude smaller. A hardcoded metre assumption passes an
/// SI-only corpus and fails here, which is why the generator emits mm.
#[test]
fn curved_elementary_surfaces_resolve_their_units_and_frames() {
    let model = fixture("synthetic_elementary_surfaces.ifc");

    match lower_one(&model, only(&model, "IFCCYLINDRICALSURFACE")) {
        GeometryNode::Surface(Surface::Cylinder(c)) => {
            assert!(
                (c.radius - 0.25).abs() < 1e-9,
                "250 mm -> 0.25 m, got {}",
                c.radius
            );
            assert!(
                (c.frame.origin.to_array()[0] - 0.1).abs() < 1e-9,
                "origin in metres"
            );
        }
        other => panic!("expected a cylinder, got {other:?}"),
    }

    match lower_one(&model, only(&model, "IFCSPHERICALSURFACE")) {
        GeometryNode::Surface(Surface::Sphere(s)) => {
            assert!(
                (s.radius - 0.18).abs() < 1e-9,
                "180 mm -> 0.18 m, got {}",
                s.radius
            );
        }
        other => panic!("expected a sphere, got {other:?}"),
    }

    match lower_one(&model, only(&model, "IFCTOROIDALSURFACE")) {
        GeometryNode::Surface(Surface::Torus(t)) => {
            assert!((t.major_radius - 0.3).abs() < 1e-9, "300 mm major");
            assert!((t.minor_radius - 0.06).abs() < 1e-9, "60 mm minor");
            assert!(t.minor_radius < t.major_radius, "a ring, not a spindle");
        }
        other => panic!("expected a torus, got {other:?}"),
    }
}

/// Each curved surface keeps the distinct placement the generator gave it.
///
/// All three sit on different axes on purpose: a fixture with everything at
/// the identity frame cannot tell a preserved frame from a dropped one.
#[test]
fn each_curved_surface_keeps_its_own_axis() {
    let model = fixture("synthetic_elementary_surfaces.ifc");
    let axis = |kind: &str| match lower_one(&model, only(&model, kind)) {
        GeometryNode::Surface(Surface::Cylinder(c)) => c.frame.z.to_array(),
        GeometryNode::Surface(Surface::Sphere(s)) => s.frame.z.to_array(),
        GeometryNode::Surface(Surface::Torus(t)) => t.frame.z.to_array(),
        other => panic!("unexpected node {other:?}"),
    };
    let cyl = axis("IFCCYLINDRICALSURFACE");
    let sph = axis("IFCSPHERICALSURFACE");
    let tor = axis("IFCTOROIDALSURFACE");
    assert!(
        (cyl[2] - 1.0).abs() < 1e-9,
        "cylinder axis is +Z, got {cyl:?}"
    );
    assert!(
        (sph[1] - 1.0).abs() < 1e-9,
        "sphere axis is +Y, got {sph:?}"
    );
    assert!((tor[0] - 1.0).abs() < 1e-9, "torus axis is +X, got {tor:?}");
}

/// The revolved surface reads its axis from the IfcAxis1Placement.
///
/// IfcSurfaceOfRevolution is the only surface family whose axis is an
/// IfcAxis1Placement rather than an IfcAxis2Placement3D. Both carry a
/// Location, so reading the wrong one still yields a surface -- one revolved
/// about the wrong line.
#[test]
fn the_revolved_surface_takes_its_axis_from_axis1_placement() {
    let model = fixture("synthetic_surface_of_revolution.ifc");
    match lower_one(&model, only(&model, "IFCSURFACEOFREVOLUTION")) {
        GeometryNode::SurfaceRelation(SurfaceRelation::Revolution {
            axis_origin,
            axis_direction,
            ..
        }) => {
            let d = axis_direction.to_array();
            assert!((d[2] - 1.0).abs() < 1e-9, "axis is +Z, got {d:?}");
            // The fixture places the axis at (40, 0, 25) millimetres, well
            // off the origin: scaling zero is still zero, so an axis through
            // the origin would hide a missing millimetre conversion entirely.
            let o = axis_origin.to_array();
            assert!(
                (o[0] - 0.040).abs() < 1e-9 && (o[2] - 0.025).abs() < 1e-9,
                "the axis origin must be converted to metres, got {o:?}"
            );
        }
        other => panic!("expected a revolution, got {other:?}"),
    }
}

/// A degree-valued trim on a revolved basis converts to radians.
///
/// The generator writes DEGREE as a conversion-based plane-angle unit and
/// trims u at 90. Applying the length factor instead yields 0.09: still a
/// valid patch, a different surface, and nothing downstream reports it.
#[test]
fn a_degree_trim_on_a_revolved_basis_becomes_radians() {
    let model = fixture("synthetic_surface_of_revolution.ifc");
    match lower_one(&model, only(&model, "IFCRECTANGULARTRIMMEDSURFACE")) {
        GeometryNode::SurfaceRelation(SurfaceRelation::RectangularTrimmed { u, v, .. }) => {
            let expected = std::f64::consts::FRAC_PI_2;
            assert!(
                (u.1 - expected).abs() < 1e-9,
                "u2 = 90 degrees must become {expected} rad, got {} (0.09 means a length factor)",
                u.1
            );
            assert!((u.0).abs() < 1e-9, "u1 stays at zero");
            assert!(v.1 > 0.0, "the v range must survive, got {}", v.1);
        }
        other => panic!("expected a rectangular trim, got {other:?}"),
    }
}

/// The B-spline patch keeps u/v degrees, knots and net orientation.
#[test]
fn the_bspline_patch_keeps_its_two_directions_apart() {
    let model = fixture("synthetic_bspline_surface.ifc");
    match lower_one(&model, only(&model, "IFCBSPLINESURFACEWITHKNOTS")) {
        GeometryNode::Surface(Surface::BSpline(p)) => {
            assert_eq!(p.u_degree, 3, "cubic along u");
            assert_eq!(p.v_degree, 1, "linear along v");
            assert_eq!(p.control_points.len(), 4, "four rows along u");
            assert_eq!(p.control_points[0].len(), 2, "two columns along v");
            assert_eq!(p.u_multiplicities, vec![4, 4], "clamped cubic");
            assert_eq!(p.v_multiplicities, vec![2, 2], "clamped linear");
            assert!(
                p.weights.is_none(),
                "polynomial patch must not gain weights"
            );
            // The saddle: row 1 column 1 is raised, row 1 column 0 is not.
            let raised = p.control_points[1][1].to_array()[2];
            let flat = p.control_points[1][0].to_array()[2];
            assert!(
                (raised - flat).abs() > 1e-6,
                "a transposed net flattens the saddle: {raised} vs {flat}"
            );
        }
        other => panic!("expected a B-spline surface, got {other:?}"),
    }
}

/// A reader-lowered spline surface is directly consumable by the scalar
/// kernel's tessellator. This is the adapter-to-kernel integration boundary:
/// IFC owns schema decoding and units; Axiolid owns evaluation and meshing.
#[test]
fn the_reader_lowered_bspline_patch_tessellates_through_the_kernel() {
    let model = fixture("synthetic_bspline_surface.ifc");
    let node = lower_one(&model, only(&model, "IFCBSPLINESURFACEWITHKNOTS"));
    let GeometryNode::Surface(surface) = node else {
        panic!("expected a surface node, got {node:?}");
    };
    let Surface::BSpline(spline) = &surface else {
        panic!("expected a B-spline surface, got {surface:?}");
    };

    let patch = Patch::new(
        *spline.u_knots.first().expect("u knot domain starts"),
        *spline.u_knots.last().expect("u knot domain ends"),
        *spline.v_knots.first().expect("v knot domain starts"),
        *spline.v_knots.last().expect("v knot domain ends"),
    )
    .expect("lowered knot domain is a valid patch");
    let budget = TessellationBudget::new(1e-4, 256).expect("finite tessellation budget");
    let outcome = tessellate_patch(&surface, patch, budget)
        .expect("the general scalar kernel evaluates the reader-lowered spline");

    assert!(
        !outcome.budget_exhausted,
        "fixture must converge within budget"
    );
    assert!(
        outcome.mesh.positions.len() >= 4,
        "surface must emit vertices"
    );
    assert!(
        outcome.mesh.indices.len() >= 6,
        "surface must emit triangles"
    );
    assert!(outcome.mesh.indices.len().is_multiple_of(3));
    assert!(
        outcome.mesh.positions.iter().all(|point| {
            let [x, y, z] = point.to_array();
            x.is_finite() && y.is_finite() && z.is_finite()
        }),
        "reader-to-kernel tessellation must stay finite"
    );
}

/// The curve-bounded plane keeps its outer boundary and its hole, in order.
#[test]
fn the_curve_bounded_plane_keeps_outer_then_hole() {
    let model = fixture("synthetic_curve_bounded_plane.ifc");
    match lower_one(&model, only(&model, "IFCCURVEBOUNDEDPLANE")) {
        GeometryNode::SurfaceRelation(SurfaceRelation::CurveBounded {
            boundaries,
            implicit_outer,
            ..
        }) => {
            assert_eq!(boundaries.len(), 2, "one outer plus one hole");
            assert!(
                !implicit_outer,
                "this family always states its outer boundary"
            );
        }
        other => panic!("expected a curve-bounded surface, got {other:?}"),
    }
}

/// Exact curve subtypes in the committed surface corpus keep their distinct
/// neutral semantics rather than merely passing dispatch.
#[test]
fn offset_intersection_and_seam_curve_semantics_survive_corpus_lowering() {
    let model = fixture("synthetic_conic_offset_bounded.ifc");

    let offset = lower_item(&model, only(&model, "IFCOFFSETCURVE2D"));
    match offset.graph.get(offset.root).expect("offset root") {
        GeometryNode::CurveRelation(CurveRelation::Offset {
            distance,
            reference_direction,
            ..
        }) => {
            assert!((*distance - 0.25).abs() < 1e-9);
            assert!(
                reference_direction.is_none(),
                "2D offsets have no reference direction"
            );
        }
        other => panic!("expected exact offset relation, got {other:?}"),
    }

    for kind in ["IFCINTERSECTIONCURVE", "IFCSEAMCURVE"] {
        let lowered = lower_item(&model, only(&model, kind));
        match lowered.graph.get(lowered.root).expect("surface-curve root") {
            GeometryNode::CurveRelation(CurveRelation::SurfaceCurve {
                associated_geometry,
                master,
                ..
            }) => {
                assert_eq!(
                    associated_geometry.len(),
                    2,
                    "{kind}: both p-curves survive"
                );
                assert_eq!(*master, MasterRepresentation::Curve3d);
            }
            other => panic!("{kind}: expected surface-curve relation, got {other:?}"),
        }
    }
}

/// The committed IFC4X3 boundary fixture uses p-curves on its common plane.
#[test]
fn the_curve_bounded_surface_corpus_keeps_parameter_curves() {
    let model = fixture("synthetic_conic_offset_bounded.ifc");
    let scale = units::resolve(&model);
    let mut session = LoweringSession::new(&model, &scale);
    let root = lower_surface_node(&mut session, EntityId(25), Transform::identity())
        .expect("corpus surface lowers");
    let lowered = session.finish(root).expect("session finishes");

    let Some(GeometryNode::SurfaceRelation(SurfaceRelation::CurveBounded {
        basis,
        boundaries,
        ..
    })) = lowered.graph.get(root)
    else {
        panic!("expected a curve-bounded surface");
    };
    assert_eq!(boundaries.len(), 2, "outer boundary plus one hole");
    for boundary in boundaries {
        let Some(GeometryNode::CurveRelation(CurveRelation::Composite { segments })) =
            lowered.graph.get(*boundary)
        else {
            panic!("expected an IFC boundary composite");
        };
        assert_eq!(segments.len(), 1);
        assert!(matches!(
            lowered.graph.get(segments[0].curve),
            Some(GeometryNode::CurveRelation(CurveRelation::ParameterCurve {
                basis_surface,
                ..
            })) if basis_surface == basis
        ));
    }
}

/// Every surface in the synthetic corpus lowers; none is silently skipped.
///
/// A loop that finds nothing passes vacuously, so the count is asserted.
#[test]
fn every_synthetic_surface_lowers() {
    let files = [
        "synthetic_elementary_surfaces.ifc",
        "synthetic_surface_of_revolution.ifc",
        "synthetic_bspline_surface.ifc",
        "synthetic_curve_bounded_plane.ifc",
    ];
    let kinds = [
        "IFCCYLINDRICALSURFACE",
        "IFCSPHERICALSURFACE",
        "IFCTOROIDALSURFACE",
        "IFCSURFACEOFREVOLUTION",
        "IFCRECTANGULARTRIMMEDSURFACE",
        "IFCBSPLINESURFACEWITHKNOTS",
        "IFCCURVEBOUNDEDPLANE",
    ];
    let mut seen = 0usize;
    for file in files {
        let model = fixture(file);
        for kind in kinds {
            for id in model.ids_of_type(kind) {
                let _ = lower_one(&model, *id);
                seen += 1;
            }
        }
    }
    assert_eq!(seen, 7, "the synthetic corpus must cover seven surfaces");
}

/// Parameter-space conics survive corpus lowering with their authored values
/// intact.
///
/// This fixture is hand-authored rather than generated: `ifcopenshell` has no
/// convenient p-curve conic emitter, and the file is small enough to read.
///
/// It is deliberately authored in **millimetres**. Every value asserted here
/// is a surface parameter, not a length, so applying the project length
/// factor would divide each one by 1000 and every assertion below would fail.
/// That is the whole point of the fixture: a corpus-level guard that the
/// parameter domain never receives a unit conversion.
#[test]
fn parameter_space_conics_keep_authored_values_through_corpus_lowering() {
    use axiolid_curve::Curve2;

    let model = fixture("synthetic_parameter_space_conics.ifc");

    let reference_curve_of = |id: EntityId| {
        let lowered = lower_item(&model, id);
        let GeometryNode::CurveRelation(CurveRelation::ParameterCurve {
            reference_curve, ..
        }) = lowered.graph.get(lowered.root).expect("pcurve root")
        else {
            panic!("expected a parameter curve relation");
        };
        lowered
            .graph
            .get(*reference_curve)
            .expect("reference curve")
            .clone()
    };

    // #14: circle, radius 1.5 in a frame at (2, 3) with default RefDirection.
    match reference_curve_of(EntityId(14)) {
        GeometryNode::Curve2(Curve2::Circle(circle)) => {
            assert_eq!(circle.radius, 1.5, "radius must not be scaled to metres");
            assert_eq!(circle.frame.origin.to_array(), [2.0, 3.0]);
            assert_eq!(circle.frame.x.to_array(), [1.0, 0.0]);
        }
        other => panic!("expected a parameter-space circle, got {other:?}"),
    }

    // #19: ellipse with RefDirection (0, 1), so local X is the global +Y axis
    // and the derived Y is its orthogonal complement (-1, 0).
    match reference_curve_of(EntityId(19)) {
        GeometryNode::Curve2(Curve2::Ellipse(ellipse)) => {
            assert_eq!(ellipse.semi_axis_x, 4.0);
            assert_eq!(ellipse.semi_axis_y, 2.5);
            assert_eq!(ellipse.frame.x.to_array(), [0.0, 1.0]);
            assert_eq!(
                ellipse.frame.y.to_array(),
                [-1.0, 0.0],
                "Y is X rotated a quarter turn counter-clockwise"
            );
        }
        other => panic!("expected a parameter-space ellipse, got {other:?}"),
    }

    // #24: line whose IfcVector carries magnitude 3, which sets the parameter
    // scale and must survive un-normalized.
    match reference_curve_of(EntityId(24)) {
        GeometryNode::Curve2(Curve2::Line(line)) => {
            assert_eq!(line.origin.to_array(), [1.0, 1.0]);
            assert_eq!(
                line.direction.to_array(),
                [3.0, 0.0],
                "the IfcVector magnitude sets the parameter scale and must not be normalized"
            );
        }
        other => panic!("expected a parameter-space line, got {other:?}"),
    }

    // #43: explicit-knot B-spline. Knots are curve parameters and control
    // points are (u, v) pairs, so neither takes the millimetre length factor.
    match reference_curve_of(EntityId(43)) {
        GeometryNode::Curve2(Curve2::BSpline(spline)) => {
            assert_eq!(spline.degree, 1);
            assert_eq!(spline.knots, vec![0.0, 1.0]);
            assert_eq!(spline.multiplicities, vec![2, 2]);
            assert_eq!(spline.control_points[0].to_array(), [1.5, 2.5]);
            assert_eq!(spline.control_points[1].to_array(), [3.5, 4.5]);
        }
        other => panic!("expected a parameter-space B-spline, got {other:?}"),
    }
}
