//! A polygonal bounded half-space clips along `Position`'s Z axis (#515).
//!
//! IFC bounds `IfcPolygonalBoundedHalfSpace` by the infinite prism of its
//! `PolygonalBoundary`, extruded along the Z axis of its own `Position`, in
//! both directions, at any angle to the plane. It lowers to Axiolid's
//! `SolidOperation::BoundedHalfSpaceAlongZ` (axiolid-model 0.3.14,
//! axiolid/kernel#321), which is that set, with `Position` carried whole.
//! Before it, the clip lowered to `BoundedHalfSpace`, whose prism runs along
//! the plane normal, and every frame that node could not hold compiled to
//! another tool or to a chain of half spaces:
//!
//! - a sloped roof plane over a vertical prism (#515): the gable wall meshed
//!   uncut, then (0.27.2) through a chain with no exact result;
//! - a plane whose normal points against `Position`'s Z: mirrored footprint;
//! - a scaled frame (a mapped item with `Scale`): the boundary kept its size;
//! - a non-convex or arc boundary under an oblique prism: refused.
//!
//! Each test asserts the analytic volume, in the mesh and, where the exact
//! compiler builds the body (every case not under a mapped item, whose
//! scaled instance it does not take), in the exact body with the mesh
//! certified against it. What the kernel refuses (a Z in the plane, a union
//! with the unbounded tool) is refused by name. None accepts the uncut or
//! mis-cut body.
#![cfg(feature = "compile-reference-backend")]

use std::path::PathBuf;

use axiolid_contracts::ExecutionOptions;
use axiolid_core::Tolerance;
use axiolid_mesh::TriMesh;
use axiolid_mesh_compile::ReferenceExactCompiler;
use axiolid_model::{GeometryNode, SolidOperation};
use ifc_geometry::compile::{self, compile_product_mesh, compile_product_mesh_net};
use ifc_geometry::error::GeometryError;
use ifc_geometry::lower::{
    lower_product_representation, LoweredGeometry, LoweringSession, RepresentationPurpose,
};
use ifc_geometry::units;
use ifc_model::{Codec, EntityId, Model};
use ifc_step::StepCodec;

/// Divergence-theorem volume, summed about the first vertex for conditioning.
fn volume(mesh: &TriMesh) -> f64 {
    assert!(!mesh.indices.is_empty(), "compiled mesh is empty");
    let base = mesh.positions[0];
    mesh.indices
        .as_chunks::<3>()
        .0
        .iter()
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

/// `product`'s Body lowered as the product entry points lower it.
fn lowered(model: &Model, product: EntityId) -> LoweredGeometry {
    let scale = units::resolve(model).expect("project units");
    let mut session = LoweringSession::new(model, &scale);
    let root = lower_product_representation(&mut session, product, RepresentationPurpose::Body)
        .expect("lowers")
        .expect("a body");
    session.finish(root).expect("a valid graph")
}

/// What the kernel certifies of `product`'s Body: the exact body's volume
/// and the mesh's deviation bound, the mesh measured against that body
/// (axiolid-mesh-compile 0.3.27 `compile_mesh_with_deviation`). Panics
/// unless the exact compiler builds it, the mesh volume is within
/// `mesh_slack` of the exact one, and the mesh is certified within the
/// requested millimetre.
fn certified(model: &Model, product: EntityId, mesh_slack: f64, case: &str) -> (f64, f64) {
    let lowered = lowered(model, product);
    let options = ExecutionOptions::new(Tolerance::MILLIMETRE);
    let (body, _) = ReferenceExactCompiler::new()
        .compile_exact_with_report(&lowered.graph, lowered.root, &options)
        .unwrap_or_else(|error| panic!("{case}: the exact compiler must build it, got {error:?}"));
    let exact = axiolid_measure::exact_properties(&body, Tolerance::MILLIMETRE)
        .expect("measurable")
        .signed_volume;
    let (outcome, report) = compile::default_backend()
        .compile_mesh_with_deviation(&lowered.graph, lowered.root, &options)
        .unwrap_or_else(|error| panic!("{case}: compiles with a report, got {error:?}"));
    let v = volume(&outcome.mesh);
    assert!(
        (v - exact).abs() <= mesh_slack,
        "{case}: mesh volume {v}, exact {exact}"
    );
    assert!(report.meets_requested(), "{case}: {report:?}");
    (exact, report.bound.expect("certified"))
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

const GABLES: [&str; 2] = [
    "gable_wall_polygonal_clip_ifc2x3.ifc",
    "gable_wall_polygonal_clip_ifc4.ifc",
];

/// The gable wall compiles to its gable, gross and net, in both releases;
/// it never meshes as the uncut extrusion (#515).
///
/// Every plane is sloped and every `Position` is axis-aligned, so each
/// prism is vertical and oblique to its plane: one `BoundedHalfSpaceAlongZ`
/// per clip, its placement `Position` as written.
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
                (v - GABLE).abs() < 1e-9,
                "{name} {path}: volume {v}, expected the gable's {GABLE}"
            );
        }
    }
}

/// Each clip is one node, no chain of half spaces (#515): four
/// `BoundedHalfSpaceAlongZ`, four planes, nothing else bounding.
#[test]
fn the_gable_wall_lowers_one_bounded_node_per_clip() {
    for name in GABLES {
        let lowered = lowered(&fixture(name), EntityId(9031));
        let nodes: Vec<&GeometryNode> = lowered.graph.iter().map(|(_, node)| node).collect();
        let along = nodes
            .iter()
            .filter(|node| {
                matches!(
                    node,
                    GeometryNode::SolidOperation(SolidOperation::BoundedHalfSpaceAlongZ { .. })
                )
            })
            .count();
        let planes = nodes
            .iter()
            .filter(|node| matches!(node, GeometryNode::HalfSpace(_)))
            .count();
        let normal = nodes
            .iter()
            .filter(|node| {
                matches!(
                    node,
                    GeometryNode::SolidOperation(SolidOperation::BoundedHalfSpace { .. })
                )
            })
            .count();
        assert_eq!((along, planes, normal), (4, 4, 0), "{name}");
    }
}

/// The exact compiler builds the gable (axiolid-mesh-compile 0.3.27), at
/// the gable's volume, and the mesh is certified against it within the
/// requested millimetre: no unbounded path any more.
#[test]
fn the_gable_wall_is_certified() {
    for name in GABLES {
        let (exact, bound) = certified(&fixture(name), EntityId(9031), 1e-9, name);
        assert!(
            (exact - GABLE).abs() < 1e-9,
            "{name}: exact volume {exact}, expected {GABLE}"
        );
        assert!(bound <= 1e-3, "{name}: bound {bound}");
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
    /// `Some(operator)`: the body is a mapped item placed by this
    /// `IfcCartesianTransformationOperator3D` (or a subtype), `#53`, its
    /// `LocalOrigin` `#6`, as a STEP line without the `#53=`.
    mapped: Option<&'static str>,
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
    mapped: None,
    boundary: RECTANGLE,
    boolean: DIFFERENCE,
};

const BOX: f64 = 48.0;

/// One proxy `#10` whose body is `#20`, a boolean of the box and PBHS #30:
/// the box x in [-2, 2], y in [-3, 3], z in [0, 2], metres.
fn clipped_box(clip: &Clip) -> String {
    let flag = if clip.agreement { ".T." } else { ".F." };
    let v = |[x, y, z]: [f64; 3]| format!("({x:?},{y:?},{z:?})");
    let (items, body, mapped) = match clip.mapped {
        None => ("'Clipping'", "#20", String::new()),
        Some(operator) => (
            "'MappedRepresentation'",
            "#50",
            format!(
                "#50=IFCMAPPEDITEM(#51,#53);
#51=IFCREPRESENTATIONMAP(#4,#52);
#52=IFCSHAPEREPRESENTATION(#2,'Body','Clipping',(#20));
#53={operator};
#54=IFCDIRECTION((1.,0.,0.));
#55=IFCDIRECTION((0.,-1.,0.));
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

fn box_model(clip: &Clip) -> Model {
    StepCodec
        .read_bytes(clipped_box(clip).as_bytes())
        .expect("fixture parses")
}

fn compile_box_mesh(clip: &Clip) -> Result<TriMesh, GeometryError> {
    compile_product_mesh(&box_model(clip), EntityId(10), Tolerance::MILLIMETRE)
        .map(|mesh| mesh.expect("the proxy has a body"))
}

fn compile_box(clip: &Clip) -> Result<f64, GeometryError> {
    compile_box_mesh(clip).map(|mesh| volume(&mesh))
}

/// The mesh is `expected` to within `slack`.
fn assert_mesh_volume(clip: &Clip, expected: f64, slack: f64, case: &str) {
    match compile_box(clip) {
        Ok(v) => assert!(
            (v - expected).abs() <= slack,
            "{case}: volume {v}, expected {expected}"
        ),
        Err(error) => panic!("{case}: the clip must compile, got {error}"),
    }
}

/// The mesh is `expected`, and so is the exact body, the mesh certified
/// against it.
fn assert_volume(clip: &Clip, expected: f64, case: &str) {
    assert_mesh_volume(clip, expected, 1e-9, case);
    let (exact, _) = certified(&box_model(clip), EntityId(10), 1e-9, case);
    assert!(
        (exact - expected).abs() < 1e-9,
        "{case}: exact volume {exact}, expected {expected}"
    );
}

/// Refused by the compiler, by a reason containing `needle`.
fn assert_refused(clip: &Clip, needle: &str, case: &str) {
    match compile_box(clip) {
        Ok(v) => panic!("{case}: must refuse, compiled volume {v}"),
        Err(GeometryError::CompilationRefused { entity, reason }) => {
            assert_eq!(entity, EntityId(10), "{case}");
            assert!(reason.contains(needle), "{case}: {reason}");
        }
        Err(error) => panic!("{case}: expected a named compiler refusal, got {error}"),
    }
}

/// The baseline, so a failure below is about the variation.
#[test]
fn the_aligned_baseline_removes_its_footprint() {
    assert_volume(&BASELINE, BOX - 0.125, "baseline");
}

/// The plane normal points DOWN, against `Position`'s Z, with `.T.`: the
/// material is still above the plane, so the clip is the baseline's set.
/// The footprint is framed by `Position`'s own X and Y, never `normal x X`.
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
    let away = Clip {
        position_axis: [0.0, 0.6, 0.8],
        ..BASELINE
    };
    assert_volume(&away, BOX, "prism tilted off the box");
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

/// The L (0, 0), (2, 0), (2, 1), (1, 1), (1, 2), (0, 2) at the box's
/// origin, under [`SLOPED`]'s plane: [0, 2] x [0, 1] loses
/// integral over [0, 2] of (0.5 + 0.75 x) dx = 2.5, [0, 1] x [1, 2] loses
/// 0.875, so 3.375 m³. Its convex hull would lose 0.5 m² more of the
/// corner (1, 1), (2, 1), (1, 2). Refused under an oblique prism up to
/// 0.27.2.
const L_SHAPE: Clip = Clip {
    origin: [0.0, 0.0, 0.0],
    boundary: "#40=IFCPOLYLINE((#41,#42,#43,#44,#45,#46,#41));
#41=IFCCARTESIANPOINT((0.,0.));
#42=IFCCARTESIANPOINT((2.,0.));
#43=IFCCARTESIANPOINT((2.,1.));
#44=IFCCARTESIANPOINT((1.,1.));
#45=IFCCARTESIANPOINT((1.,2.));
#46=IFCCARTESIANPOINT((0.,2.));
",
    ..SLOPED
};

const L_CUT: f64 = 3.375;

/// A non-convex boundary under an oblique prism cuts its own L (#515).
#[test]
fn a_non_convex_boundary_under_an_oblique_prism_cuts_its_l() {
    assert_volume(&L_SHAPE, BOX - L_CUT, "L-shaped boundary");
    let clockwise = Clip {
        boundary: "#40=IFCPOLYLINE((#41,#46,#45,#44,#43,#42,#41));
#41=IFCCARTESIANPOINT((0.,0.));
#42=IFCCARTESIANPOINT((2.,0.));
#43=IFCCARTESIANPOINT((2.,1.));
#44=IFCCARTESIANPOINT((1.,1.));
#45=IFCCARTESIANPOINT((1.,2.));
#46=IFCCARTESIANPOINT((0.,2.));
",
        ..L_SHAPE
    };
    assert_volume(&clockwise, BOX - L_CUT, "clockwise L");
}

/// The unit square [0, 1]² closed on the right by the half disc about
/// (1, 0.5) of radius 0.5, at the box's origin, under [`SLOPED`]'s plane.
/// The square loses 0.875; the half disc, area pi / 8 with its centroid at
/// x = 1 + 2 / (3 pi), loses pi / 8 (0.5 + 0.75 x) = 1.25 pi / 8 + 1 / 16.
/// Its prism is vertical, so the arc's wall is a right cylinder the sloped
/// plane cuts in an ellipse. Refused under an oblique prism up to 0.27.2.
const ARC: Clip = Clip {
    origin: [0.0, 0.0, 0.0],
    boundary: "#40=IFCINDEXEDPOLYCURVE(#41,(IFCLINEINDEX((1,2)),IFCARCINDEX((2,3,4)),\
IFCLINEINDEX((4,5,1))),$);
#41=IFCCARTESIANPOINTLIST2D(((0.,0.),(1.,0.),(1.5,0.5),(1.,1.),(0.,1.)),$);
",
    ..SLOPED
};

fn arc_cut() -> f64 {
    0.875 + 1.25 * std::f64::consts::PI / 8.0 + 1.0 / 16.0
}

/// The chord slack of a mesh cut by [`ARC`]: within a millimetre of the
/// cylinder over its 0.79 m arc and at most 1.3 m of height.
const ARC_MESH_SLACK: f64 = 2e-3;

/// An arc boundary under an oblique prism cuts its cylinder exactly in the
/// exact compiler, and within the certified chord in the mesh (#515).
#[test]
fn an_arc_boundary_under_an_oblique_prism_cuts_its_cylinder() {
    let expected = BOX - arc_cut();
    assert_mesh_volume(&ARC, expected, ARC_MESH_SLACK, "arc boundary");
    let (exact, bound) = certified(&box_model(&ARC), EntityId(10), ARC_MESH_SLACK, "arc");
    assert!(
        (exact - expected).abs() < 1e-9,
        "arc: exact volume {exact}, expected {expected}"
    );
    assert!(bound <= 1e-3, "arc: bound {bound}");
}

/// A mapped item with `Scale` 2 scales the whole clip, sloped or not: the
/// box becomes 384 m³ and every cut 8 times its own. The placement reaches
/// the node whole (axiolid/kernel#322), so the boundary is scaled with it.
#[test]
fn a_scaled_mapped_clip_scales_its_boundary() {
    let scale = Some("IFCCARTESIANTRANSFORMATIONOPERATOR3D($,$,#6,2.,$)");
    for (case, clip, cut) in [
        ("scaled", BASELINE, 0.125),
        ("scaled and sloped", SLOPED, SLOPED_CUT),
        ("scaled L", L_SHAPE, L_CUT),
    ] {
        let scaled = Clip {
            mapped: scale,
            ..clip
        };
        assert_mesh_volume(&scaled, 8.0 * (BOX - cut), 1e-9, case);
    }
}

/// A mapped item scaled 2 in y only: every volume doubles, the L's and the
/// rectangle's cuts with it, and the arc's half disc becomes a half
/// ellipse, its wall an elliptic cylinder the mesh compiler cuts. (The
/// exact compiler takes no scaled instance, so these meshes, like every
/// scaled mapped item's, carry no certified bound.)
#[test]
fn an_unevenly_scaled_clip_stretches_its_boundary() {
    let stretch = Some("IFCCARTESIANTRANSFORMATIONOPERATOR3DNONUNIFORM($,$,#6,1.,$,2.,1.)");
    for (case, clip, cut, slack) in [
        ("stretched", BASELINE, 0.125, 1e-9),
        ("stretched and sloped", SLOPED, SLOPED_CUT, 1e-9),
        ("stretched L", L_SHAPE, L_CUT, 1e-9),
        ("stretched arc", ARC, arc_cut(), 2.0 * ARC_MESH_SLACK),
    ] {
        let stretched = Clip {
            mapped: stretch,
            ..clip
        };
        assert_mesh_volume(&stretched, 2.0 * (BOX - cut), slack, case);
    }
}

/// A mapped item mirrored in y (`Axis2` -Y) mirrors the whole clip: the
/// volume is the unmirrored one, and the cut lands on the mirrored side,
/// y in [-3, -2.75], never back on +y (axiolid/kernel#322 reads the
/// placement's handedness).
#[test]
fn a_mirrored_mapped_clip_mirrors_its_boundary() {
    let mirror = Some("IFCCARTESIANTRANSFORMATIONOPERATOR3D($,#55,#6,1.,$)");
    for (case, clip, cut) in [
        ("mirrored", BASELINE, 0.125),
        ("mirrored and sloped", SLOPED, SLOPED_CUT),
    ] {
        let mirrored = Clip {
            mapped: mirror,
            ..clip
        };
        let mesh = compile_box_mesh(&mirrored)
            .unwrap_or_else(|error| panic!("{case}: must compile, got {error}"));
        let v = volume(&mesh);
        assert!((v - (BOX - cut)).abs() < 1e-9, "{case}: volume {v}");
        let at = |y: f64| mesh.positions.iter().any(|p| (p.y - y).abs() < 1e-9);
        assert!(at(-2.75), "{case}: the cut must reach y = -2.75");
        assert!(!at(2.75), "{case}: the cut must not stay at y = +2.75");
    }
}

/// `Position`'s Z lying in the plane: its prism never crosses the plane.
/// Lowered as written; the kernel refuses it by name.
#[test]
fn a_prism_in_the_plane_is_refused_by_name() {
    let vertical_plane = Clip {
        plane_axis: [1.0, 0.0, 0.0],
        ..BASELINE
    };
    assert_refused(&vertical_plane, "parallel to the clip plane", "Z in plane");
}

/// A union with the tool is unbounded on the tool's kept side: refused by
/// the kernel by name, never meshed as some finite slab.
#[test]
fn a_union_with_the_tool_is_refused_by_name() {
    let union = Clip {
        boolean: "IFCBOOLEANRESULT(.UNION.",
        ..SLOPED
    };
    assert_refused(&union, "half-space", "union");
}
