//! A polygonal bounded half-space clip lowers AND compiles (#45).
//!
//! Lowering tests pinned the node shape and passed while the reference
//! compiler refused every such clip: the boundary was a `Curve3`, the kernel
//! contract is `Curve2`. Only compiling end to end catches that class of
//! drift, so this test goes file -> lowered graph -> mesh -> volume.
//!
//! # The geometry, and why each number is what it is
//!
//! The body is a 4 x 6 x 2 box (x in [-2, 2], y in [-3, 3], z in [0, 2]),
//! volume 48. The clip plane sits at z = 1.5 with `AgreementFlag = .F.`, so
//! the cutting material is ABOVE the plane (0.5 thick). The boundary is the
//! rectangle x in [1, 4], y in [0, 0.5] authored in `Position`'s own XY
//! plane; it overhangs the box, so the removed prism is its footprint clipped
//! to the box, times the height.
//!
//! The box is not square and the boundary is off its own origin, so every
//! plausible bug moves the volume to a different value instead of passing by
//! coincidence:
//!
//! - polarity flipped (material below the plane): the height becomes 1.5,
//! - `Position`'s rotation ignored: the long side stays on x,
//! - `Position`'s translation ignored: the footprint stays at the plane origin,
//! - unit factor not applied to the boundary: the millimetre footprint lands
//!   a kilometre away and removes nothing.
#![cfg(feature = "compile-reference-backend")]

use axiolid_core::Tolerance;
use axiolid_mesh::TriMesh;
use ifc_geometry::compile::compile_product_mesh;
use ifc_model::{Codec, EntityId};
use ifc_step::StepCodec;

/// The unclipped box volume.
const BOX: f64 = 48.0;

/// How `Position` is authored relative to the clip plane.
struct Clip {
    /// Length factor written into the file: 1 for metres, 1000 for mm.
    unit: f64,
    /// `Position`'s X axis; its Z axis is +Z.
    ref_direction: [f64; 3],
    /// `Position`'s in-plane origin. The clip plane passes through (0, 0).
    origin: [f64; 2],
    /// `AgreementFlag`.
    agreement: bool,
}

const METRES: Clip = Clip {
    unit: 1.0,
    ref_direction: [1.0, 0.0, 0.0],
    origin: [0.0, 0.0],
    agreement: false,
};

/// One proxy whose body is `IfcBooleanClippingResult(box, PBHS)`.
///
/// Every length is multiplied by `clip.unit`, so each file describes the same
/// solid in its own unit.
fn clipped_box(clip: &Clip) -> String {
    let prefix = if clip.unit == 1.0 { "$" } else { ".MILLI." };
    let l = |v: f64| format!("{:?}", v * clip.unit);
    let flag = if clip.agreement { ".T." } else { ".F." };
    let [rx, ry, rz] = clip.ref_direction;
    let [ox, oy] = clip.origin;
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
#5=IFCSIUNIT(*,.LENGTHUNIT.,{prefix},.METRE.);
#6=IFCCARTESIANPOINT((0.,0.,0.));
#7=IFCDIRECTION((0.,0.,1.));
#10=IFCBUILDINGELEMENTPROXY('1YvctVUKr0kugbFTf53O9L',$,'Clipped',$,$,#11,#12,$,$);
#11=IFCLOCALPLACEMENT($,#4);
#12=IFCPRODUCTDEFINITIONSHAPE($,$,(#13));
#13=IFCSHAPEREPRESENTATION(#2,'Body','Clipping',(#20));
#20=IFCBOOLEANCLIPPINGRESULT(.DIFFERENCE.,#21,#30);
#21=IFCEXTRUDEDAREASOLID(#22,#4,#7,{depth});
#22=IFCRECTANGLEPROFILEDEF(.AREA.,$,#23,{x_dim},{y_dim});
#23=IFCAXIS2PLACEMENT2D(#24,$);
#24=IFCCARTESIANPOINT((0.,0.));
#30=IFCPOLYGONALBOUNDEDHALFSPACE(#31,{flag},#34,#40);
#31=IFCPLANE(#32);
#32=IFCAXIS2PLACEMENT3D(#33,$,$);
#33=IFCCARTESIANPOINT((0.,0.,{plane_z}));
#34=IFCAXIS2PLACEMENT3D(#35,#7,#36);
#35=IFCCARTESIANPOINT(({ox},{oy},0.));
#36=IFCDIRECTION(({rx:?},{ry:?},{rz:?}));
#40=IFCPOLYLINE((#41,#42,#43,#44,#41));
#41=IFCCARTESIANPOINT(({x0},{y0}));
#42=IFCCARTESIANPOINT(({x1},{y0}));
#43=IFCCARTESIANPOINT(({x1},{y1}));
#44=IFCCARTESIANPOINT(({x0},{y1}));
ENDSEC;
END-ISO-10303-21;
",
        depth = l(2.0),
        x_dim = l(4.0),
        y_dim = l(6.0),
        plane_z = l(1.5),
        ox = l(ox),
        oy = l(oy),
        x0 = l(1.0),
        x1 = l(4.0),
        y0 = l(0.0),
        y1 = l(0.5),
    )
}

/// Compile product `#10` and return its enclosed volume in cubic metres.
fn compiled_volume(clip: &Clip) -> f64 {
    let text = clipped_box(clip);
    let model = StepCodec
        .read_bytes(text.as_bytes())
        .expect("fixture parses");
    let mesh = compile_product_mesh(&model, EntityId(10), Tolerance::MILLIMETRE)
        .unwrap_or_else(|error| panic!("the clip must compile, got {error}"))
        .expect("the product has a body");
    signed_volume(&mesh)
}

/// Divergence-theorem volume, summed about the first vertex for conditioning.
fn signed_volume(mesh: &TriMesh) -> f64 {
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

fn assert_removed(clip: &Clip, removed: f64, case: &str) {
    let volume = compiled_volume(clip);
    assert!(
        (volume - (BOX - removed)).abs() < 1e-6,
        "{case}: volume {volume}, expected {} (removed {removed})",
        BOX - removed
    );
}

/// Footprint x in [1, 2] (clipped from [1, 4]), y in [0, 0.5] -> area 0.5,
/// removed 0.5 x 0.5 = 0.25.
#[test]
fn a_polygonal_bounded_clip_compiles_to_the_analytic_volume() {
    assert_removed(&METRES, 0.25, "metres, unrotated");
}

/// `Position` rotated a quarter turn about Z: local x -> world y, local y ->
/// world -x. Footprint x in [-0.5, 0], y in [1, 3] (clipped from [1, 4]) ->
/// area 1.0, removed 0.5. Ignoring the rotation removes 0.25.
#[test]
fn the_boundary_follows_the_in_plane_rotation_of_position() {
    let rotated = Clip {
        ref_direction: [0.0, 1.0, 0.0],
        ..METRES
    };
    assert_removed(&rotated, 0.5, "metres, rotated");
}

/// `.T.` puts the cutting material below the plane: height 1.5, so the
/// footprint removes 0.5 x 1.5 = 0.75.
#[test]
fn the_agreement_flag_selects_the_side_below_the_plane() {
    let below = Clip {
        agreement: true,
        ..METRES
    };
    assert_removed(&below, 0.75, "metres, agreement .T.");
}

/// The same solid authored in millimetres compiles to the same metres.
#[test]
fn a_millimetre_file_clips_the_same_solid() {
    let millimetres = Clip {
        unit: 1000.0,
        ..METRES
    };
    assert_removed(&millimetres, 0.25, "millimetres, unrotated");
}

/// `Position` translated to (0, 2.75) within the clip plane: footprint
/// x in [1, 2], y in [2.75, 3] (clipped from [2.75, 3.25]) -> area 0.25,
/// removed 0.125. Ignoring the translation removes 0.25.
///
/// Lowering carries the translation on `BoundedHalfSpace.placement`.
/// `axiolid-construct` 0.3.0 anchored the boundary at the clip plane's origin
/// instead and compiled the wrong volume with no error (axiolid/kernel#164);
/// 0.3.1, the floor this workspace pins, places it correctly.
#[test]
fn the_boundary_follows_the_in_plane_translation_of_position() {
    let translated = Clip {
        origin: [0.0, 2.75],
        ..METRES
    };
    assert_removed(&translated, 0.125, "metres, translated");
}
