//! Curved advanced B-rep faces bounded by 3D edge curves compile (#454).
//!
//! IFC writes an `IfcAdvancedFace`'s boundary as `IfcEdgeCurve`s whose
//! `EdgeGeometry` is a 3D curve: IFC4 ADD2 TC1 and IFC4X3 ADD2 admit only
//! `IfcLine`, `IfcConic`, `IfcPolyline` and `IfcBSplineCurve` there
//! (`IfcAdvancedFace.ApplicableEdgeCurves`), never an `IfcPcurve`. The
//! lowering keeps that curve as the edge's carrier and attaches no
//! parameter-space curve; projecting edges onto surfaces is computation,
//! which ADR 0004 keeps out of the bridge. Up to `axiolid-mesh-compile`
//! 0.3.25 the reference compiler refused such a curved face; 0.3.26 inverts
//! each edge's shared 3D samples onto the face's surface (closed form on
//! analytic surfaces, certified on B-splines) and meshes a plane bounded by
//! curved edges through the same samples (axiolid/kernel#310).
//!
//! The fixtures (`test/fixtures/synthetic-surfaces/advanced_brep_3d_edges_*.ifc`)
//! state one model in IFC4 (millimetres) and IFC4X3_ADD2 (metres):
//!
//! - `CYLINDER`: radius 0.5, height 2, placed at `(3, 4, 0)`, as an
//!   advanced B-rep of a seamed `IfcCylindricalSurface` wall and two
//!   `IfcPlane` caps; the rims are closed `IfcCircle` edges, the seam an
//!   `IfcLine` whose vector spans the height. Volume `pi r^2 h`.
//! - `PILLOW`: `[0, 2] x [0, 1.5]` placed at `(0, -4, 0)`, its top a
//!   biquadratic `IfcBSplineSurfaceWithKnots` `z = 1 + 1.2 u (1 - u) +
//!   0.8 v (1 - v)` (`x = 2u`, `y = 1.5v`) bounded by four
//!   `IfcBSplineCurveWithKnots` edges, its four walls planes bounded by one
//!   of those curves and three `IfcPolyline` edges. Volume
//!   `3 (1 + 0.2 + 0.4 / 3) = 4`.
//!
//! IFC2X3 has no `IfcAdvancedBrep`, so there is no IFC2X3 twin.
//!
//! The kernel's refusals of 3D-bounded faces it cannot parametrise reach
//! the caller as `GeometryError::CompilationRefused`, naming the reason.

#![cfg(feature = "compile-reference-backend")]

use std::f64::consts::PI;
use std::path::PathBuf;

use axiolid_contracts::ExecutionOptions;
use axiolid_core::Tolerance;
use axiolid_mesh::TriMesh;
use axiolid_mesh_compile_contract::MeshClosure;
use ifc_geometry::compile::{self, compile_product_mesh, compile_product_mesh_reported};
use ifc_geometry::lower::{lower_product_representation, LoweringSession, RepresentationPurpose};
use ifc_geometry::{units, GeometryError};
use ifc_model::{Codec, EntityId, Model, Value};
use ifc_step::StepCodec;

const RELEASES: [&str; 2] = ["ifc4", "ifc4x3"];

fn text(release: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../test/fixtures/synthetic-surfaces")
        .join(format!("advanced_brep_3d_edges_{release}.ifc"));
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn parse(text: &str) -> Model {
    StepCodec
        .read_bytes(text.as_bytes())
        .expect("fixture parses")
}

/// The fixture of `release` with each `(record, replacement)` applied; each
/// record is unique.
fn edited(release: &str, edits: &[(&str, &str)]) -> Model {
    let mut text = text(release);
    for (record, replacement) in edits {
        assert_eq!(text.matches(record).count(), 1, "{record} is unique");
        text = text.replace(record, replacement);
    }
    parse(&text)
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

/// The compiled solid of `name`, asserted closed and declared a solid.
fn solid(model: &Model, name: &str) -> TriMesh {
    let compiled =
        compile_product_mesh_reported(model, product(model, name), Tolerance::MILLIMETRE)
            .unwrap_or_else(|error| panic!("{name} compiles: {error}"))
            .expect("the product has a body");
    assert_eq!(compiled.closure, MeshClosure::Solid, "{name}");
    let health = axiolid_mesh::audit::audit_mesh(&compiled.mesh, Tolerance::MILLIMETRE);
    assert!(
        health.is_closed_two_manifold(),
        "{name} is watertight: {health:?}"
    );
    compiled.mesh
}

fn signed_volume(mesh: &TriMesh) -> f64 {
    let base = mesh.positions[0];
    mesh.indices
        .chunks_exact(3)
        .map(|t| {
            let [a, b, c] = [t[0], t[1], t[2]].map(|i| mesh.positions[i as usize] - base);
            a.dot(b.cross(c)) / 6.0
        })
        .sum()
}

fn bounds(mesh: &TriMesh) -> ([f64; 3], [f64; 3]) {
    let mut lo = [f64::INFINITY; 3];
    let mut hi = [f64::NEG_INFINITY; 3];
    for p in &mesh.positions {
        for (axis, value) in [p.x, p.y, p.z].into_iter().enumerate() {
            lo[axis] = lo[axis].min(value);
            hi[axis] = hi[axis].max(value);
        }
    }
    (lo, hi)
}

fn assert_bounds(name: &str, mesh: &TriMesh, lo: [f64; 3], hi: [f64; 3], slack: f64) {
    let (got_lo, got_hi) = bounds(mesh);
    for axis in 0..3 {
        assert!(
            (got_lo[axis] - lo[axis]).abs() <= slack && (got_hi[axis] - hi[axis]).abs() <= slack,
            "{name}: bounds {got_lo:?}..{got_hi:?}, expected {lo:?}..{hi:?}"
        );
    }
}

/// The deviation report of `name`'s body, compiled as the product entry
/// points compile it.
fn report(model: &Model, name: &str) -> axiolid_mesh_compile::DeviationReport {
    let scale = units::resolve(model);
    let mut session = LoweringSession::new(model, &scale);
    let root = lower_product_representation(
        &mut session,
        product(model, name),
        RepresentationPurpose::Body,
    )
    .expect("lowers")
    .expect("a body");
    let lowered = session.finish(root).expect("a valid graph");
    let (_, report) = compile::default_backend()
        .compile_mesh_with_deviation(
            &lowered.graph,
            lowered.root,
            &ExecutionOptions::new(Tolerance::MILLIMETRE),
        )
        .unwrap_or_else(|e| panic!("{name} compiles with a report: {e:?}"));
    report
}

/// The cylinder closes as a solid of `pi r^2 h`. Its rims are chorded
/// inside the exact circles, so the mesh holds less than the exact volume,
/// by no more than its surface area times the 1 mm chord budget.
#[test]
fn the_seamed_cylinder_compiles_to_its_volume() {
    let (r, h) = (0.5_f64, 2.0_f64);
    let exact = PI * r * r * h;
    let area = 2.0 * PI * r * h + 2.0 * PI * r * r;
    for release in RELEASES {
        let model = parse(&text(release));
        let mesh = solid(&model, "CYLINDER");
        let volume = signed_volume(&mesh);
        assert!(
            volume <= exact && exact - volume <= area * 1e-3,
            "{release}: volume {volume}, exact {exact}"
        );
        assert_bounds(
            release,
            &mesh,
            [3.0 - r, 4.0 - r, 0.0],
            [3.0 + r, 4.0 + r, h],
            1e-3,
        );
    }
}

/// The B-spline pillow closes as a solid of 4. Its top bulges upward, so
/// the chorded mesh lies under it and holds less than the exact volume, by
/// no more than its surface area times the budget.
#[test]
fn the_bspline_pillow_compiles_to_its_volume() {
    let exact = 4.0;
    let area_bound = 2.0 * (2.0 * 1.5) + 2.0 * (2.0 + 1.5) * 1.5 + 2.0;
    for release in RELEASES {
        let model = parse(&text(release));
        let mesh = solid(&model, "PILLOW");
        let volume = signed_volume(&mesh);
        assert!(
            volume <= exact + 1e-9 && exact - volume <= area_bound * 1e-3,
            "{release}: volume {volume}, exact {exact}"
        );
        assert_bounds(release, &mesh, [0.0, -4.0, 0.0], [2.0, -2.5, 1.5], 1e-3);
    }
}

/// The cylinder's report is certified within the 1 mm request. Its wall
/// is bounded as a cylinder and its caps as planes, the caps by the rims'
/// chord bounds shared with the wall.
#[test]
fn the_cylinder_is_certified_within_the_request() {
    for release in RELEASES {
        let report = report(&parse(&text(release)), "CYLINDER");
        assert_eq!(report.requested, 1e-3);
        assert!(report.meets_requested(), "{release}: {report:?}");
        assert_eq!(report.unbounded().count(), 0, "{release}: {report:?}");
        let details: Vec<&str> = report.contributions.iter().map(|c| c.detail).collect();
        assert!(
            details.contains(&"cylinder") && details.contains(&"plane"),
            "{release}: {report:?}"
        );
    }
}

/// The pillow's report is certified, but slightly ABOVE the 1 mm request:
/// a curved face's bound adds its triangles' bound to the trim terms of its
/// inverted boundary, so it may exceed the request (axiolid/kernel#310).
/// This pins the kernel's answer rather than assuming the request is met:
/// `meets_requested` is false, and the bound stays within a tenth of the
/// request above it.
#[test]
fn the_pillow_is_certified_slightly_above_the_request() {
    for release in RELEASES {
        let report = report(&parse(&text(release)), "PILLOW");
        let bound = report.bound.expect("certified");
        assert!(!report.meets_requested(), "{release}: {report:?}");
        assert!(
            bound > report.requested && bound <= 1.1 * report.requested,
            "{release}: {report:?}"
        );
        assert!(
            report
                .contributions
                .iter()
                .any(|c| c.detail == "B-spline surface"),
            "{release}: {report:?}"
        );
    }
}

/// An `IfcLine` seam whose vector is a unit length, as many exporters write
/// it, ends a millimetre above the bottom rim rather than at the top
/// vertex. Without a p-curve the kernel reads the curve's whole domain as
/// the edge, so it refuses the edge by name.
#[test]
fn a_seam_line_that_does_not_end_at_its_vertex_is_refused_by_name() {
    let model = edited(
        "ifc4",
        &[("#117=IFCVECTOR(#101,2000.);", "#117=IFCVECTOR(#101,1.);")],
    );
    let id = product(&model, "CYLINDER");
    let error = compile_product_mesh(&model, id, Tolerance::MILLIMETRE)
        .expect_err("the seam does not end at its vertex");
    match error {
        GeometryError::CompilationRefused { entity, reason } => {
            assert_eq!(entity, id);
            assert!(reason.contains("from its vertex"), "{reason}");
        }
        other => panic!("expected CompilationRefused, got {other:?}"),
    }
}

/// The pillow's south edge with its middle control point raised 0.1 m
/// stays in the plane `y = 0` of its wall but leaves the B-spline top by
/// up to 0.05 m. Its samples have no parameters on the top surface, and the
/// compilation is refused rather than meshed with a gap. The kernel's
/// certified inversion gives up as a spent budget, not as an off-surface
/// sample: `BudgetExceeded` naming the certified surface projection, a
/// typed refusal in its own right, which this pins as reported.
#[test]
fn an_edge_off_its_bspline_face_is_refused() {
    let model = edited(
        "ifc4",
        &[(
            "#222=IFCBSPLINECURVEWITHKNOTS(2,(#200,#203,#206)",
            "#330=IFCCARTESIANPOINT((1000.,0.,1700.));\n\
             #222=IFCBSPLINECURVEWITHKNOTS(2,(#200,#330,#206)",
        )],
    );
    let id = product(&model, "PILLOW");
    let error = compile_product_mesh(&model, id, Tolerance::MILLIMETRE)
        .expect_err("the edge leaves its surface");
    match error {
        GeometryError::CompilationRefused { entity, reason } => {
            assert_eq!(entity, id);
            assert!(
                reason.contains("BudgetExceeded")
                    && reason.contains("certified surface projection"),
                "{reason}"
            );
        }
        other => panic!("expected CompilationRefused, got {other:?}"),
    }
}

/// A hemisphere whose dome is seamed by a meridian up to the pole.
const HEMISPHERE: &str = "ISO-10303-21;
HEADER;
FILE_DESCRIPTION(('ViewDefinition [DesignTransferView]'),'2;1');
FILE_NAME('hemisphere.ifc','2026-10-14T00:00:00',(''),(''),'','','');
FILE_SCHEMA(('IFC4'));
ENDSEC;
DATA;
#1=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.);
#2=IFCUNITASSIGNMENT((#1));
#3=IFCCARTESIANPOINT((0.,0.,0.));
#4=IFCAXIS2PLACEMENT3D(#3,$,$);
#5=IFCGEOMETRICREPRESENTATIONCONTEXT($,'Model',3,1.E-05,#4,$);
#6=IFCPROJECT('2X1JuxwDb1D8__KlJU0pdt',$,'Hemisphere',$,$,$,$,(#5),#2);
#10=IFCDIRECTION((0.,0.,1.));
#11=IFCDIRECTION((1.,0.,0.));
#12=IFCAXIS2PLACEMENT3D(#3,#10,#11);
#13=IFCSPHERICALSURFACE(#12,1.);
#14=IFCDIRECTION((0.,0.,-1.));
#15=IFCAXIS2PLACEMENT3D(#3,#14,#11);
#16=IFCPLANE(#15);
#20=IFCCARTESIANPOINT((1.,0.,0.));
#21=IFCCARTESIANPOINT((1.,0.,1.));
#22=IFCCARTESIANPOINT((0.,0.,1.));
#23=IFCVERTEXPOINT(#20);
#24=IFCVERTEXPOINT(#22);
#25=IFCCIRCLE(#12,1.);
#26=IFCRATIONALBSPLINECURVEWITHKNOTS(2,(#20,#21,#22),.CIRCULAR_ARC.,.F.,.F.,(3,3),(0.,1.),.UNSPECIFIED.,(1.,0.7071067811865476,1.));
#27=IFCEDGECURVE(#23,#23,#25,.T.);
#28=IFCEDGECURVE(#23,#24,#26,.T.);
#30=IFCORIENTEDEDGE(*,*,#27,.T.);
#31=IFCORIENTEDEDGE(*,*,#28,.T.);
#32=IFCORIENTEDEDGE(*,*,#28,.F.);
#33=IFCEDGELOOP((#30,#31,#32));
#34=IFCFACEOUTERBOUND(#33,.T.);
#35=IFCADVANCEDFACE((#34),#13,.T.);
#36=IFCORIENTEDEDGE(*,*,#27,.F.);
#37=IFCEDGELOOP((#36));
#38=IFCFACEOUTERBOUND(#37,.T.);
#39=IFCADVANCEDFACE((#38),#16,.T.);
#40=IFCCLOSEDSHELL((#35,#39));
#41=IFCADVANCEDBREP(#40);
#42=IFCSHAPEREPRESENTATION(#5,'Body','AdvancedBrep',(#41));
#43=IFCPRODUCTDEFINITIONSHAPE($,$,(#42));
#44=IFCLOCALPLACEMENT($,#4);
#45=IFCBUILDINGELEMENTPROXY('2d0mYI$KLFzfXYI_pPyijw',$,'HEMISPHERE',$,$,#44,#43,$,$);
ENDSEC;
END-ISO-10303-21;
";

/// A meridian edge up to the pole of an `IfcSphericalSurface` has no
/// determined parameters there: every longitude names the pole. The kernel
/// refuses it by name rather than guess one.
#[test]
fn an_edge_through_a_sphere_pole_is_refused_by_name() {
    let model = parse(HEMISPHERE);
    let id = product(&model, "HEMISPHERE");
    let error = compile_product_mesh(&model, id, Tolerance::MILLIMETRE)
        .expect_err("the meridian reaches the pole");
    match error {
        GeometryError::CompilationRefused { entity, reason } => {
            assert_eq!(entity, id);
            assert!(reason.contains("sphere pole"), "{reason}");
        }
        other => panic!("expected CompilationRefused, got {other:?}"),
    }
}
