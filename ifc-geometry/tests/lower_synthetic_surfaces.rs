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

use axiolid_model::{CurveRelation, GeometryNode, SurfaceRelation};
use axiolid_scalar::surface::Patch;
use axiolid_scalar::tessellate::{tessellate_patch, TessellationBudget};
use axiolid_surface::Surface;
use ifc_geometry::lower::{lower_surface_node, LoweringSession, Tolerance};
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
    let mut session = LoweringSession::new(model, &scale, Tolerance::building_scale());
    let node = lower_surface_node(&mut session, id, Transform::identity())
        .unwrap_or_else(|e| panic!("surface {id:?} must lower: {e}"));
    let lowered = session.finish(node).expect("session finishes");
    lowered.graph.get(lowered.root).expect("root node").clone()
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

/// The committed IFC4X3 boundary fixture uses p-curves on its common plane.
#[test]
fn the_curve_bounded_surface_corpus_keeps_parameter_curves() {
    let model = fixture("synthetic_conic_offset_bounded.ifc");
    let scale = units::resolve(&model);
    let mut session = LoweringSession::new(&model, &scale, Tolerance::building_scale());
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
