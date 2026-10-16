//! A polygonal bounded half-space clips along `Position`'s Z axis, or refuses
//! by name (#515).
//!
//! IFC bounds `IfcPolygonalBoundedHalfSpace` by the infinite prism of its
//! `PolygonalBoundary`, extruded along the Z axis of its own `Position`, in
//! both directions. Axiolid's `SolidOperation::BoundedHalfSpace` sweeps its
//! boundary along the clip plane's NORMAL instead, framing it by the
//! placement's X axis projected into the plane and `normal x X`. The two agree
//! only when `Position`'s XY frame is an unscaled right-handed frame about the
//! normal. Every other case used to lower anyway and compile to a different
//! tool, without an error:
//!
//! - a sloped roof plane over a vertical prism (#515): the boundary was laid
//!   into the slope and mirrored off the wall, so the gable wall meshed uncut;
//! - a plane whose normal points against `Position`'s Z, the usual way an
//!   exporter writes "remove what is above": the footprint was mirrored;
//! - a scaled frame (a mapped item with `Scale`): the boundary kept its size.
//!
//! The first two lower to the kernel's bounded node, reoriented; the
//! oblique and rescaled ones lower, as a boolean's tool, to an intersection
//! of unbounded half spaces when the boundary is a convex polygon. Each test
//! asserts the analytic volume where the clip is lowered, and a refusal
//! naming the half-space where it is not (a non-convex or arc boundary, a
//! union). None accepts the uncut or mis-cut body.
#![cfg(feature = "compile-reference-backend")]

use std::path::PathBuf;

use axiolid_contracts::{ExecutionOptions, GeomError};
use axiolid_core::Tolerance;
use axiolid_mesh::TriMesh;
use axiolid_mesh_compile::ReferenceExactCompiler;
use ifc_geometry::compile::{self, compile_product_mesh, compile_product_mesh_net};
use ifc_geometry::error::GeometryError;
use ifc_geometry::lower::{lower_product_representation, LoweringSession, RepresentationPurpose};
use ifc_geometry::units;
use ifc_model::{Codec, EntityId, Model};
use ifc_step::StepCodec;

/// Divergence-theorem volume, summed about the first vertex for conditioning.
fn volume(mesh: &TriMesh) -> f64 {
    assert!(!mesh.indices.is_empty(), "compiled mesh is empty");
    let base = mesh.positions[0];
    mesh.indices
        .chunks_exact(3)
        .map(|t| {
            let [a, b, c] = [t[0], t[1], t[2]].map(|i| mesh.positions[i as usize] - base);
            a.dot(b.cross(c)) / 6.0
        })
        .sum()
}

fn fixture(name: &str) -> Model {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../test/fixtures/synthetic-compile")
        .join(name);
    StepCodec.read_path(&path).expect("fixture parses")
}

/// The gable wall's volume in m³: the 350 mm thick elevation polygon
/// (0, 0), (9775, 0), (9775, 11809.43), (9299.31, 13536.68),
/// (4925, 16165.03), (550.69, 13536.68), (0, 11537.10), its top on the four
/// planes. Derived from the planes in the file (each through its `Location`,
/// normal its `Axis`) and matched by IfcOpenShell 0.8.5 (14 vertices,
/// 49.999912 m³).
const GABLE: f64 = 49.999_912_042_537;

/// The uncut extrusion: 9.775 x 0.35 x 16.165033 m.
const UNCUT: f64 = 55.304_619_372_356;

/// The gable wall compiles to its gable, gross and net, in both releases;
/// it never meshes as the uncut extrusion (#515).
///
/// Every plane is sloped and every `Position` is axis-aligned, so each
/// prism is vertical and oblique to its plane. The neutral bounded half-space
/// sweeps along the plane normal and cannot hold that prism: the kernel's
/// mesh was the boundary laid into the slope and mirrored to y in
/// [-525, -175] mm, beside the wall, so the four differences removed
/// nothing. Each clip now lowers to the wall minus its intersection with the
/// plane's half space and the four sides of its rectangle's prism.
#[test]
fn the_gable_wall_is_clipped_to_its_gable() {
    for name in GABLES {
        let model = fixture(name);
        let gross = compile_product_mesh(&model, EntityId(9031), Tolerance::MILLIMETRE);
        let net = compile_product_mesh_net(&model, EntityId(9031), Tolerance::MILLIMETRE)
            .map(|net| net.map(|net| net.mesh));
        for (path, result) in [("gross", gross), ("net", net)] {
            let mesh = result
                .unwrap_or_else(|error| panic!("{name} {path}: must compile, got {error}"))
                .unwrap_or_else(|| panic!("{name} {path}: the wall has a Body"));
            let v = volume(&mesh);
            assert!(
                (v - UNCUT).abs() > 1e-3,
                "{name} {path}: the clips were dropped, volume {v} is the uncut body"
            );
            assert!(
                (v - GABLE).abs() < 1e-6,
                "{name} {path}: volume {v}, expected the gable's {GABLE}"
            );
        }
    }
}

const GABLES: [&str; 2] = [
    "gable_wall_polygonal_clip_ifc2x3.ifc",
    "gable_wall_polygonal_clip_ifc4.ifc",
];

/// The wall lowered as the product entry points lower it.
fn lowered_gable(model: &Model) -> ifc_geometry::lower::LoweredGeometry {
    let scale = units::resolve(model);
    let mut session = LoweringSession::new(model, &scale);
    let root =
        lower_product_representation(&mut session, EntityId(9031), RepresentationPurpose::Body)
            .expect("lowers")
            .expect("a body");
    session.finish(root).expect("a valid graph")
}

/// What the kernel says of the gable beyond its volume.
///
/// The mesh compiler measures a boolean against the exact compiler's result
/// for it (axiolid-mesh-compile 0.3.26 `compile_mesh_with_deviation`). The
/// exact compiler clips by half spaces, but its general boolean takes only a
/// placed extrusion as a difference's tool, so `S - (S ∩ T)` has no exact
/// result there yet and the mesh's bound is unbounded under a name, never
/// invented. Should the kernel build the exact result, it must be the
/// gable's volume, and the mesh must be certified within the budget.
#[test]
fn the_gable_wall_is_certified_or_names_the_missing_exact_result() {
    for name in GABLES {
        let model = fixture(name);
        let lowered = lowered_gable(&model);
        let options = ExecutionOptions::new(Tolerance::MILLIMETRE);
        let (outcome, report) = compile::default_backend()
            .compile_mesh_with_deviation(&lowered.graph, lowered.root, &options)
            .unwrap_or_else(|error| panic!("{name}: compiles with a report, got {error:?}"));
        let v = volume(&outcome.mesh);
        assert!(
            (v - GABLE).abs() < 1e-6,
            "{name}: mesh volume {v}, expected {GABLE}"
        );
        match ReferenceExactCompiler::new().compile_exact_with_report(
            &lowered.graph,
            lowered.root,
            &options,
        ) {
            Ok((body, _)) => {
                let exact = axiolid_measure::exact_properties(&body, Tolerance::MILLIMETRE)
                    .expect("measurable")
                    .signed_volume;
                assert!(
                    (exact - GABLE).abs() < 1e-6,
                    "{name}: exact volume {exact}, expected {GABLE}"
                );
                assert!(report.meets_requested(), "{name}: {report:?}");
            }
            Err(GeomError::UnsupportedInput { .. }) => {
                assert!(report.bound.is_none(), "{name}: {report:?}");
                assert!(
                    report.unbounded().count() > 0,
                    "{name}: an unbounded path must be named: {report:?}"
                );
            }
            Err(error) => panic!("{name}: the exact compiler must build or refuse, got {error:?}"),
        }
    }
}

fn assert_refused_clip(error: &GeometryError, clips: &[u64], case: &str) {
    match error {
        GeometryError::Unsupported {
            entity, type_name, ..
        } => {
            assert!(
                clips.contains(&entity.0),
                "{case}: refused {entity}, not a clip: {error}"
            );
            assert_eq!(type_name, "IFCPOLYGONALBOUNDEDHALFSPACE", "{case}: {error}");
        }
        other => panic!("{case}: expected an Unsupported refusal naming a clip, got {other}"),
    }
}

/// How the clip of a 4 x 6 x 2 m box is authored. See [`clipped_box`].
#[derive(Clone, Copy)]
struct Clip {
    /// The clip plane's `Axis` (its normal); the plane passes through
    /// (0, 0, 1.5).
    plane_axis: [f64; 3],
    /// `Position`'s `Axis` and `RefDirection`.
    position_axis: [f64; 3],
    position_ref: [f64; 3],
    /// `Position`'s origin.
    origin: [f64; 3],
    /// `AgreementFlag`.
    agreement: bool,
    /// `Some(scale)`: the body is a mapped item with this uniform `Scale`.
    mapped_scale: Option<f64>,
    /// `PolygonalBoundary` `#40` and what it references, as STEP lines.
    boundary: &'static str,
    /// The boolean `#20`, its operator included.
    boolean: &'static str,
}

/// The x in [1, 4], y in [0, 0.5] rectangle, counter-clockwise.
const RECTANGLE: &str = "#40=IFCPOLYLINE((#41,#42,#43,#44,#41));
#41=IFCCARTESIANPOINT((1.,0.));
#42=IFCCARTESIANPOINT((4.,0.));
#43=IFCCARTESIANPOINT((4.,0.5));
#44=IFCCARTESIANPOINT((1.,0.5));
";

const DIFFERENCE: &str = "IFCBOOLEANCLIPPINGRESULT(.DIFFERENCE.";

/// The `bounded_halfspace_compile.rs` baseline: a horizontal plane, normal
/// +Z, `.F.` (material above), `Position` at (0, 2.75) unrotated. The
/// boundary x in [1, 4], y in [0, 0.5] covers x in [1, 2], y in [2.75, 3] of
/// the box: 0.25 m², 0.5 m high, removing 0.125 m³.
const BASELINE: Clip = Clip {
    plane_axis: [0.0, 0.0, 1.0],
    position_axis: [0.0, 0.0, 1.0],
    position_ref: [1.0, 0.0, 0.0],
    origin: [0.0, 2.75, 0.0],
    agreement: false,
    mapped_scale: None,
    boundary: RECTANGLE,
    boolean: DIFFERENCE,
};

const BOX: f64 = 48.0;

/// One proxy `#10` whose body is `#20`, a boolean of the box and PBHS #30:
/// the box x in [-2, 2], y in [-3, 3], z in [0, 2], metres.
fn clipped_box(clip: &Clip) -> String {
    let flag = if clip.agreement { ".T." } else { ".F." };
    let v = |[x, y, z]: [f64; 3]| format!("({x:?},{y:?},{z:?})");
    let (items, body, mapped) = match clip.mapped_scale {
        None => ("'Clipping'", "#20", String::new()),
        Some(scale) => (
            "'MappedRepresentation'",
            "#50",
            format!(
                "#50=IFCMAPPEDITEM(#51,#53);
#51=IFCREPRESENTATIONMAP(#4,#52);
#52=IFCSHAPEREPRESENTATION(#2,'Body','Clipping',(#20));
#53=IFCCARTESIANTRANSFORMATIONOPERATOR3D($,$,#6,{scale:?},$);
"
            ),
        ),
    };
    format!(
        "ISO-10303-21;
HEADER;
FILE_DESCRIPTION((''),'2;1');
FILE_NAME('','',(''),(''),'','','');
FILE_SCHEMA(('IFC4'));
ENDSEC;
DATA;
#1=IFCPROJECT('0YvctVUKr0kugbFTf53O9L',$,'P',$,$,$,$,(#2),#3);
#2=IFCGEOMETRICREPRESENTATIONCONTEXT($,'Model',3,1.E-05,#4,$);
#3=IFCUNITASSIGNMENT((#5));
#4=IFCAXIS2PLACEMENT3D(#6,$,$);
#5=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.);
#6=IFCCARTESIANPOINT((0.,0.,0.));
#7=IFCDIRECTION((0.,0.,1.));
#10=IFCBUILDINGELEMENTPROXY('1YvctVUKr0kugbFTf53O9L',$,'Clipped',$,$,#11,#12,$,$);
#11=IFCLOCALPLACEMENT($,#4);
#12=IFCPRODUCTDEFINITIONSHAPE($,$,(#13));
#13=IFCSHAPEREPRESENTATION(#2,'Body',{items},({body}));
#20={boolean},#21,#30);
#21=IFCEXTRUDEDAREASOLID(#22,#4,#7,2.);
#22=IFCRECTANGLEPROFILEDEF(.AREA.,$,#23,4.,6.);
#23=IFCAXIS2PLACEMENT2D(#24,$);
#24=IFCCARTESIANPOINT((0.,0.));
#30=IFCPOLYGONALBOUNDEDHALFSPACE(#31,{flag},#34,#40);
#31=IFCPLANE(#32);
#32=IFCAXIS2PLACEMENT3D(#33,#38,$);
#33=IFCCARTESIANPOINT((0.,0.,1.5));
#34=IFCAXIS2PLACEMENT3D(#35,#37,#36);
#35=IFCCARTESIANPOINT({origin});
#36=IFCDIRECTION({position_ref});
#37=IFCDIRECTION({position_axis});
#38=IFCDIRECTION({plane_axis});
{boundary}{mapped}ENDSEC;
END-ISO-10303-21;
",
        origin = v(clip.origin),
        position_ref = v(clip.position_ref),
        position_axis = v(clip.position_axis),
        plane_axis = v(clip.plane_axis),
        boundary = clip.boundary,
        boolean = clip.boolean,
    )
}

fn compile_box(clip: &Clip) -> Result<f64, GeometryError> {
    let text = clipped_box(clip);
    let model = StepCodec
        .read_bytes(text.as_bytes())
        .expect("fixture parses");
    compile_product_mesh(&model, EntityId(10), Tolerance::MILLIMETRE)
        .map(|mesh| volume(&mesh.expect("the proxy has a body")))
}

fn assert_volume(clip: &Clip, expected: f64, case: &str) {
    match compile_box(clip) {
        Ok(v) => assert!(
            (v - expected).abs() < 1e-6,
            "{case}: volume {v}, expected {expected}"
        ),
        Err(error) => panic!("{case}: the clip must compile, got {error}"),
    }
}

/// Refused by an `Unsupported` naming `#30` whose detail contains `needle`.
fn assert_refused(clip: &Clip, needle: &str, case: &str) {
    match compile_box(clip) {
        Ok(v) => panic!("{case}: must refuse, compiled volume {v}"),
        Err(error) => {
            assert_refused_clip(&error, &[30], case);
            assert!(error.to_string().contains(needle), "{case}: {error}");
        }
    }
}

/// The baseline, so a failure below is about the variation.
#[test]
fn the_aligned_baseline_removes_its_footprint() {
    assert_volume(&BASELINE, BOX - 0.125, "baseline");
}

/// The plane normal points DOWN, against `Position`'s Z, with `.T.`: the
/// material is still above the plane, so the clip is the baseline's set.
/// The kernel frames the boundary as `(X, normal x X)`, which for a downward
/// normal is `(X, -Y)`: the footprint was mirrored to y in [2.25, 2.75] and
/// removed 0.25 instead of 0.125.
#[test]
fn a_normal_against_position_z_keeps_the_footprint_unmirrored() {
    let down = Clip {
        plane_axis: [0.0, 0.0, -1.0],
        agreement: true,
        ..BASELINE
    };
    assert_volume(&down, BOX - 0.125, "normal against Position Z");
}

/// `Position` turned upside down (Z down, X along +X, so Y along -Y): the
/// boundary's y now runs to -Y, footprint y in [2.25, 2.75], all inside the
/// box: 0.5 m² x 0.5 m = 0.25 removed. The prism is infinite both ways, so
/// Z's sign does not matter; Y's does.
#[test]
fn an_inverted_position_frames_the_boundary_by_its_own_axes() {
    let inverted = Clip {
        position_axis: [0.0, 0.0, -1.0],
        ..BASELINE
    };
    assert_volume(&inverted, BOX - 0.25, "Position upside down");
    let both = Clip {
        position_axis: [0.0, 0.0, -1.0],
        plane_axis: [0.0, 0.0, -1.0],
        agreement: true,
        ..BASELINE
    };
    assert_volume(&both, BOX - 0.25, "Position and normal both down");
}

/// The baseline under a plane sloped to z = 1.5 - 0.75 x, material above:
/// the vertical prism over x in [1, 2], y in [2.75, 3] loses
/// 0.25 * integral over [1, 2] of (0.5 + 0.75 x) dx = 0.40625 m³.
const SLOPED: Clip = Clip {
    plane_axis: [0.6, 0.0, 0.8],
    ..BASELINE
};

/// What [`SLOPED`] removes.
const SLOPED_CUT: f64 = 0.406_25;

/// A sloped plane over a vertical prism, the #515 shape: the prism is cut
/// as authored, along `Position`'s Z, whichever way the boundary winds and
/// whatever collinear vertices it repeats. IfcOpenShell 0.8.5 agrees on the
/// rectangle (47.59375).
#[test]
fn a_sloped_plane_over_a_vertical_prism_cuts_the_prism() {
    assert_volume(&SLOPED, BOX - SLOPED_CUT, "sloped plane");
    let clockwise = Clip {
        boundary: "#40=IFCPOLYLINE((#41,#44,#43,#42,#41));
#41=IFCCARTESIANPOINT((1.,0.));
#42=IFCCARTESIANPOINT((4.,0.));
#43=IFCCARTESIANPOINT((4.,0.5));
#44=IFCCARTESIANPOINT((1.,0.5));
",
        ..SLOPED
    };
    assert_volume(&clockwise, BOX - SLOPED_CUT, "clockwise boundary");
    let collinear = Clip {
        boundary: "#40=IFCPOLYLINE((#41,#45,#42,#43,#44,#41));
#41=IFCCARTESIANPOINT((1.,0.));
#42=IFCCARTESIANPOINT((4.,0.));
#43=IFCCARTESIANPOINT((4.,0.5));
#44=IFCCARTESIANPOINT((1.,0.5));
#45=IFCCARTESIANPOINT((2.5,0.));
",
        ..SLOPED
    };
    assert_volume(&collinear, BOX - SLOPED_CUT, "collinear vertex");
    // The triangle (1, 0), (4, 0), (1, 0.25): y up to (4 - x) / 12 over
    // x in [1, 2], so it loses integral of (4 - x) / 12 (0.5 + 0.75 x) dx.
    let triangle = Clip {
        boundary: "#40=IFCPOLYLINE((#41,#42,#43,#41));
#41=IFCCARTESIANPOINT((1.,0.));
#42=IFCCARTESIANPOINT((4.,0.));
#43=IFCCARTESIANPOINT((1.,0.25));
",
        ..SLOPED
    };
    assert_volume(&triangle, BOX - 1.0 / 3.0, "triangle");
}

/// A horizontal plane over a prism tilted in y. With `Axis` (0, -0.6, 0.8)
/// and `RefDirection` +X, `Position`'s Y is (0, 0.8, 0.6), and the prism's
/// section at height z is x in [1, 2] (within the box), y from
/// 2.75 - 0.75 z, 0.625 wide: 0.625 m² over z in [1.5, 2], 0.3125 m³.
/// Tilted the other way, (0, 0.6, 0.8), the section starts at
/// y = 2.75 + 0.75 z, beyond the box, and nothing is removed. IfcOpenShell
/// 0.8.5 agrees on both (47.6875 and 48).
#[test]
fn a_tilted_prism_cuts_its_own_section() {
    let tilted = Clip {
        position_axis: [0.0, -0.6, 0.8],
        ..BASELINE
    };
    assert_volume(&tilted, BOX - 0.3125, "tilted prism");
    // The kernel's mesh compiler refuses a half-space boolean whose subject
    // is empty, which the chain's next tool meets once the prism misses the
    // box. Never another volume.
    let away = Clip {
        position_axis: [0.0, 0.6, 0.8],
        ..BASELINE
    };
    match compile_box(&away) {
        Ok(v) => assert!(
            (v - BOX).abs() < 1e-6,
            "prism tilted off the box: volume {v}"
        ),
        Err(GeometryError::CompilationRefused { entity, .. }) => assert_eq!(entity, EntityId(10)),
        Err(error) => {
            panic!("prism tilted off the box: expected the box or a refusal, got {error}")
        }
    }
}

/// The intersection with the tool is the tool's part of the box.
#[test]
fn an_oblique_intersection_keeps_the_prism_part() {
    let kept = Clip {
        boolean: "IFCBOOLEANRESULT(.INTERSECTION.",
        ..SLOPED
    };
    assert_volume(&kept, SLOPED_CUT, "intersection");
}

/// What an intersection of half spaces cannot hold is refused by name,
/// never approximated: a non-convex boundary (its prism is a union), an
/// arc (its wall is a cylinder), and a union (unbounded).
#[test]
fn an_oblique_clip_beyond_half_spaces_is_refused_by_name() {
    let l_shape = Clip {
        boundary: "#40=IFCPOLYLINE((#41,#42,#43,#44,#45,#46,#41));
#41=IFCCARTESIANPOINT((1.,0.));
#42=IFCCARTESIANPOINT((4.,0.));
#43=IFCCARTESIANPOINT((4.,0.5));
#44=IFCCARTESIANPOINT((2.,0.5));
#45=IFCCARTESIANPOINT((2.,0.25));
#46=IFCCARTESIANPOINT((1.,0.25));
",
        ..SLOPED
    };
    assert_refused(&l_shape, "non-convex", "L-shaped boundary");
    let arc = Clip {
        boundary: "#40=IFCINDEXEDPOLYCURVE(#41,(IFCLINEINDEX((1,2)),IFCARCINDEX((2,3,4)),\
IFCLINEINDEX((4,5,1))),$);
#41=IFCCARTESIANPOINTLIST2D(((1.,0.),(4.,0.),(4.25,0.25),(4.,0.5),(1.,0.5)),$);
",
        ..SLOPED
    };
    assert_refused(&arc, "circular arc", "arc boundary");
    let union = Clip {
        boolean: "IFCBOOLEANRESULT(.UNION.",
        ..SLOPED
    };
    assert_refused(&union, "oblique", "union");
}

/// A mapped item with `Scale` 2 scales the whole clip: the box becomes
/// 384 m³ and the removed prism 8 x 0.125 = 1.0. The kernel's bounded node
/// normalises the placement's axes, so the boundary would keep its unscaled
/// size; the clip lowers to half spaces instead, each placed by the scaled
/// frame, sloped or not.
#[test]
fn a_scaled_mapped_clip_scales_its_boundary() {
    let scaled = Clip {
        mapped_scale: Some(2.0),
        ..BASELINE
    };
    assert_volume(&scaled, 8.0 * BOX - 1.0, "scaled");
    let sloped = Clip {
        mapped_scale: Some(2.0),
        ..SLOPED
    };
    assert_volume(&sloped, 8.0 * (BOX - SLOPED_CUT), "scaled and sloped");
}
