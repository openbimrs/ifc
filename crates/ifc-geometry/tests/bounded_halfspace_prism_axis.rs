//! A polygonal bounded half-space clips along `Position`'s Z axis, or refuses
//! (#515).
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
//! Each test asserts the analytic volume where the kernel can represent the
//! clip, and a refusal naming the half-space where it cannot. None accepts
//! the uncut or mis-cut body.
#![cfg(feature = "compile-reference-backend")]

use std::path::PathBuf;

use axiolid_core::Tolerance;
use axiolid_mesh::TriMesh;
use ifc_geometry::compile::{compile_product_mesh, compile_product_mesh_net};
use ifc_geometry::error::GeometryError;
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

/// The four `IfcPolygonalBoundedHalfSpace` clips of the gable wall.
const GABLE_CLIPS: [u64; 4] = [8990, 9002, 9014, 9026];

/// The gable wall's volume in m³: the 350 mm thick elevation polygon
/// (0, 0), (9775, 0), (9775, 11809.43), (9299.31, 13536.68),
/// (4925, 16165.03), (550.69, 13536.68), (0, 11537.10), its top on the four
/// planes. Derived from the planes in the file (each through its `Location`,
/// normal its `Axis`) and matched by IfcOpenShell 0.8.5 (14 vertices,
/// 49.999912 m³).
const GABLE: f64 = 49.999_912_042_537;

/// The uncut extrusion: 9.775 x 0.35 x 16.165033 m.
const UNCUT: f64 = 55.304_619_372_356;

/// The gable wall compiles to its gable, or refuses by naming a clip; it
/// never meshes as the uncut extrusion (#515).
///
/// Every plane is sloped and every `Position` is axis-aligned, so each
/// prism is vertical and oblique to its plane. The neutral bounded half-space
/// sweeps along the plane normal and cannot hold that prism: the kernel's
/// mesh was the boundary laid into the slope and mirrored to y in
/// [-525, -175] mm, beside the wall, so the four differences removed nothing.
#[test]
fn the_gable_wall_is_clipped_or_refused_never_uncut() {
    for name in [
        "gable_wall_polygonal_clip_ifc2x3.ifc",
        "gable_wall_polygonal_clip_ifc4.ifc",
    ] {
        let model = fixture(name);
        let gross = compile_product_mesh(&model, EntityId(9031), Tolerance::MILLIMETRE);
        let net = compile_product_mesh_net(&model, EntityId(9031), Tolerance::MILLIMETRE)
            .map(|net| net.map(|net| net.mesh));
        for (path, result) in [("gross", gross), ("net", net)] {
            match result {
                Ok(Some(mesh)) => {
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
                Ok(None) => panic!("{name} {path}: the wall has a Body"),
                Err(error) => assert_refused_clip(&error, &GABLE_CLIPS, name),
            }
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
}

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
};

const BOX: f64 = 48.0;

/// One proxy `#10` whose body is `IfcBooleanClippingResult(box, PBHS #30)`:
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
#20=IFCBOOLEANCLIPPINGRESULT(.DIFFERENCE.,#21,#30);
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
#40=IFCPOLYLINE((#41,#42,#43,#44,#41));
#41=IFCCARTESIANPOINT((1.,0.));
#42=IFCCARTESIANPOINT((4.,0.));
#43=IFCCARTESIANPOINT((4.,0.5));
#44=IFCCARTESIANPOINT((1.,0.5));
{mapped}ENDSEC;
END-ISO-10303-21;
",
        origin = v(clip.origin),
        position_ref = v(clip.position_ref),
        position_axis = v(clip.position_axis),
        plane_axis = v(clip.plane_axis),
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

fn assert_refused(clip: &Clip, case: &str) {
    match compile_box(clip) {
        Ok(v) => panic!("{case}: must refuse, compiled volume {v}"),
        Err(error) => assert_refused_clip(&error, &[30], case),
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

/// A prism oblique to the plane cannot be held by the neutral node, whose
/// prism is along the plane normal: refused by name, never approximated.
#[test]
fn a_prism_oblique_to_the_plane_is_refused() {
    let sloped_plane = Clip {
        plane_axis: [0.6, 0.0, 0.8],
        ..BASELINE
    };
    assert_refused(&sloped_plane, "sloped plane, vertical prism");
    let tilted_prism = Clip {
        position_axis: [0.0, 0.6, 0.8],
        position_ref: [1.0, 0.0, 0.0],
        ..BASELINE
    };
    assert_refused(&tilted_prism, "horizontal plane, tilted prism");
}

/// A mapped item with `Scale` 2 scales the whole clip: the box becomes
/// 384 m³ and the removed prism 8 x 0.125 = 1.0. The kernel normalises the
/// placement's axes, so the boundary would keep its unscaled size: this
/// either compiles to the scaled volume or refuses, never the mix.
#[test]
fn a_scaled_mapped_clip_scales_its_boundary_or_refuses() {
    let scaled = Clip {
        mapped_scale: Some(2.0),
        ..BASELINE
    };
    match compile_box(&scaled) {
        Ok(v) => assert!(
            (v - (8.0 * BOX - 1.0)).abs() < 1e-6,
            "scaled: volume {v}, expected {}",
            8.0 * BOX - 1.0
        ),
        Err(error) => assert_refused_clip(&error, &[30], "scaled"),
    }
}
