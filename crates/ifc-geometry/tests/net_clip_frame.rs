//! A Body with a boolean or a clip is lowered in its product's frame, even
//! without openings (#655).
//!
//! The wall: an `IfcExtrudedAreaSolid`, 2.5 x 0.125, 3.6 high, clipped by an
//! `IfcBooleanClippingResult` whose `IfcPolygonalBoundedHalfSpace` has its
//! plane at z = 2.7, normal -Z, and a 0.1 x 0.125 boundary at x 1.1..1.2: a
//! slot exactly as thick as the wall, flush with both its faces. Lowered in
//! world coordinates with the wall turned 10 degrees about Z, the faces that
//! coincide in the wall's frame end up about 1e-16 apart, and the reference
//! kernel keeps a lid over the slot on one face (axioval/engine#373). In
//! the wall's frame, with one `Instance` above, the slot is open.
#![cfg(feature = "compile-reference-backend")]

use axiolid_core::{BooleanOperator, Point3, Tolerance, Transform3};
use axiolid_mesh::TriMesh;
use axiolid_mesh_boolean_boolmesh::BoolmeshBoolean;
use axiolid_mesh_compile::ReferenceMeshCompiler;
use axiolid_mesh_compile_contract::MeshClosure;
use axiolid_model::{GeometryNode, SolidOperation};
use ifc_geometry::compile::compile_product_mesh_net_with;
use ifc_geometry::lower::{
    lower_product_net, product_world_transform, LoweredGeometry, LoweringSession,
};
use ifc_geometry::units;
use ifc_model::{Codec, EntityId, Model};
use ifc_step::StepCodec;

/// The wall.
const WALL: EntityId = EntityId(10);
/// 2.5 x 0.125 x 3.6, less the slot 0.1 x 0.125 x 0.9.
const NET: f64 = 2.5 * 0.125 * 3.6 - 0.1 * 0.125 * 0.9;

/// What the wall's Body holds.
#[derive(Clone, Copy, Debug)]
enum Body {
    /// The clipping result itself.
    Clip,
    /// An `IfcMappedItem` whose mapped representation holds the clip.
    MappedClip,
    /// The extrusion and a second one standing on it: no boolean.
    Plain,
}

/// Where the turned walls stand.
const AT: [f64; 3] = [13.475, -15.95, 4.3];

/// A file with the wall `#10` placed at `at`, turned `degrees` about Z.
fn wall(at: [f64; 3], degrees: f64, body: Body) -> Model {
    let [x, y, z] = at;
    let (sin, cos) = degrees.to_radians().sin_cos();
    let (kind, items) = match body {
        Body::Clip => ("Clipping", "#33"),
        Body::MappedClip => ("MappedRepresentation", "#40"),
        Body::Plain => ("SweptSolid", "#17,#44"),
    };
    let data = format!(
        "#1=IFCPROJECT('0YvctVUKr0kugbFTf53O9L',$,'P',$,$,$,$,(#2),#3);
#2=IFCGEOMETRICREPRESENTATIONCONTEXT($,'Model',3,1.E-05,#4,$);
#3=IFCUNITASSIGNMENT((#5,#8));
#4=IFCAXIS2PLACEMENT3D(#6,$,$);
#5=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.);
#6=IFCCARTESIANPOINT((0.,0.,0.));
#7=IFCDIRECTION((0.,0.,1.));
#8=IFCSIUNIT(*,.PLANEANGLEUNIT.,$,.RADIAN.);
#10=IFCWALL('1YvctVUKr0kugbFTf53O9L',$,'Wall',$,$,#11,#12,$,$);
#11=IFCLOCALPLACEMENT($,#13);
#12=IFCPRODUCTDEFINITIONSHAPE($,$,(#16));
#13=IFCAXIS2PLACEMENT3D(#14,#7,#15);
#14=IFCCARTESIANPOINT(({x:?},{y:?},{z:?}));
#15=IFCDIRECTION(({cos:?},{sin:?},0.));
#16=IFCSHAPEREPRESENTATION(#2,'Body','{kind}',({items}));
#17=IFCEXTRUDEDAREASOLID(#18,#4,#7,3.6);
#18=IFCRECTANGLEPROFILEDEF(.AREA.,$,#19,2.5,0.125);
#19=IFCAXIS2PLACEMENT2D(#20,$);
#20=IFCCARTESIANPOINT((1.25,0.0625));
#21=IFCDIRECTION((0.,0.,-1.));
#22=IFCDIRECTION((-1.,0.,0.));
#23=IFCCARTESIANPOINT((0.,0.,2.7));
#24=IFCAXIS2PLACEMENT3D(#23,#21,#22);
#25=IFCPLANE(#24);
#26=IFCCARTESIANPOINT((1.1,0.));
#27=IFCCARTESIANPOINT((1.2,0.));
#28=IFCCARTESIANPOINT((1.2,0.125));
#29=IFCCARTESIANPOINT((1.1,0.125));
#31=IFCPOLYLINE((#26,#27,#28,#29,#26));
#32=IFCPOLYGONALBOUNDEDHALFSPACE(#25,.T.,#4,#31);
#33=IFCBOOLEANCLIPPINGRESULT(.DIFFERENCE.,#17,#32);
#40=IFCMAPPEDITEM(#41,#43);
#41=IFCREPRESENTATIONMAP(#4,#42);
#42=IFCSHAPEREPRESENTATION(#2,'Body','Clipping',(#33));
#43=IFCCARTESIANTRANSFORMATIONOPERATOR3D($,$,#6,$,$);
#44=IFCEXTRUDEDAREASOLID(#18,#45,#7,0.5);
#45=IFCAXIS2PLACEMENT3D(#46,$,$);
#46=IFCCARTESIANPOINT((0.,0.,3.6));
"
    );
    let text = format!(
        "ISO-10303-21;
HEADER;
FILE_DESCRIPTION((''),'2;1');
FILE_NAME('','',(''),(''),'','','');
FILE_SCHEMA(('IFC4'));
ENDSEC;
DATA;
{data}ENDSEC;
END-ISO-10303-21;
"
    );
    StepCodec
        .read_bytes(text.as_bytes())
        .expect("the wall parses")
}

/// The wall's net graph and its world transform.
fn lowered(model: &Model) -> (LoweredGeometry, Transform3) {
    let scale = units::resolve(model).expect("project units");
    let mut session = LoweringSession::new(model, &scale);
    let net = lower_product_net(&mut session, WALL)
        .expect("lowers")
        .expect("has a Body");
    assert!(net.subtractions.is_empty(), "nothing voids the wall");
    assert_eq!(net.root, net.gross, "with nothing to cut, net is gross");
    let world = product_world_transform(model, &scale, WALL)
        .expect("placed")
        .to_geom();
    (session.finish(net.root).expect("finishes"), world)
}

fn is_difference(node: Option<&GeometryNode>) -> bool {
    matches!(
        node,
        Some(GeometryNode::SolidOperation(SolidOperation::Boolean {
            operator: BooleanOperator::Difference,
            ..
        }))
    )
}

/// The turned clipped wall: one Instance of the wall's world transform,
/// the clip's Difference below it, in the wall's frame.
#[test]
fn a_clip_runs_in_the_wall_frame_under_one_instance() {
    let (lowered, world) = lowered(&wall(AT, 10.0, Body::Clip));
    let Some(GeometryNode::Instance(instance)) = lowered.graph.get(lowered.root) else {
        panic!("the net root is not an Instance");
    };
    assert_eq!(
        instance.transform, world,
        "the Instance carries the wall's world transform"
    );
    assert!(
        is_difference(lowered.graph.get(instance.source)),
        "the Instance's source is the clip"
    );
}

/// A clip inside a mapped item already runs in the map's frame, under the
/// mapped item's Instance; that lowering is unchanged.
#[test]
fn a_mapped_clip_keeps_its_instance() {
    let (lowered, world) = lowered(&wall(AT, 10.0, Body::MappedClip));
    let Some(GeometryNode::Instance(instance)) = lowered.graph.get(lowered.root) else {
        panic!("the net root is not an Instance");
    };
    assert_eq!(instance.transform, world);
    assert!(
        matches!(
            lowered.graph.get(instance.source),
            Some(GeometryNode::Collection(members))
                if members.len() == 1 && is_difference(lowered.graph.get(members[0]))
        ),
        "the mapped representation's one item is the clip"
    );
}

/// Upright at the origin the wall's frame is the world: no Instance, the
/// clip's Difference is the root, as before.
#[test]
fn a_clip_at_the_origin_needs_no_instance() {
    let (lowered, world) = lowered(&wall([0.0; 3], 0.0, Body::Clip));
    assert_eq!(world, Transform3::IDENTITY);
    assert!(
        is_difference(lowered.graph.get(lowered.root)),
        "the root is the clip"
    );
}

/// A Body without a boolean is lowered in world coordinates as before.
#[test]
fn a_body_without_a_boolean_keeps_the_world_frame() {
    let (lowered, _) = lowered(&wall(AT, 10.0, Body::Plain));
    assert!(
        matches!(
            lowered.graph.get(lowered.root),
            Some(GeometryNode::Collection(_))
        ),
        "the root is the Collection of both world-placed extrusions, got {:?}",
        lowered.graph.get(lowered.root)
    );
}

/// Triangles whose centroid, in the wall's frame, lies inside the slot's
/// mouth (x 1.1..1.2, above z 2.7): a lid over the slot.
fn lid_triangles(mesh: &TriMesh, world: Transform3) -> usize {
    let local = world.inverse();
    mesh.indices
        .as_chunks::<3>()
        .0
        .iter()
        .filter(|t| {
            let c = t
                .iter()
                .map(|&i| local.transform_point3(mesh.positions[i as usize]))
                .fold(Point3::ZERO, |sum, p| sum + p)
                / 3.0;
            c.x > 1.1 + 1e-6 && c.x < 1.2 - 1e-6 && c.z > 2.7 + 1e-6
        })
        .count()
}

fn volume(mesh: &TriMesh) -> f64 {
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

/// The issue's table: upright and turned, the slot is open and the wall a
/// closed solid of the net volume.
#[test]
fn the_turned_clipped_wall_meshes_without_a_lid() {
    let backend = ReferenceMeshCompiler::new(BoolmeshBoolean);
    for (degrees, body) in [
        (0.0, Body::Clip),
        (10.0, Body::Clip),
        (10.0, Body::MappedClip),
    ] {
        let model = wall(AT, degrees, body);
        let scale = units::resolve(&model).expect("project units");
        let world = product_world_transform(&model, &scale, WALL)
            .expect("placed")
            .to_geom();
        let net = compile_product_mesh_net_with(&backend, &model, WALL, Tolerance::MILLIMETRE)
            .unwrap_or_else(|error| panic!("{degrees} {body:?}: must mesh, got {error}"))
            .expect("has a Body");
        assert_eq!(net.closure, MeshClosure::Solid, "{degrees} {body:?}");
        assert!(
            axiolid_mesh::audit::audit_mesh(&net.mesh, Tolerance::ZERO).is_closed_two_manifold(),
            "{degrees} {body:?}: not a closed two-manifold"
        );
        assert!(
            (volume(&net.mesh) - NET).abs() < 1e-9,
            "{degrees} {body:?}: volume {}",
            volume(&net.mesh)
        );
        assert_eq!(
            lid_triangles(&net.mesh, world),
            0,
            "{degrees} {body:?}: a lid covers the slot"
        );
    }
}
