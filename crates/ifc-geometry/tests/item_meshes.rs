//! A product's Body meshed one leaf item at a time (#656).
//!
//! A window is a frame and a pane, and a viewer colours them differently. The
//! per-product mesh is one lump, so `compile_product_item_meshes` returns one
//! gross mesh per leaf item, in the order the item colours come.
//!
//! # The geometry
//!
//! The window #10 lists its pane before its frame, so authored order is not
//! id order. The frame #30 is a 1 x 0.1 profile extruded 1.2 m (0.12 m3,
//! x in [-0.5, 0.5], y in [-0.05, 0.05], z in [0, 1.2]). The pane #40 is a
//! 0.8 x 0.01 profile extruded 1 m, placed at (0, 0.5, 0.1) (0.008 m3,
//! y in [0.495, 0.505], z in [0.1, 1.1]), disjoint from the frame.
#![cfg(feature = "compile-reference-backend")]

use std::sync::atomic::{AtomicUsize, Ordering};

use axiolid_contracts::{Backend, BackendDescriptor, ExecutionOptions, GeomError, GeomResult};
use axiolid_core::Tolerance;
use axiolid_mesh::TriMesh;
use axiolid_mesh_compile_contract::MeshCompiler;
use axiolid_model::{GeometryGraph, GeometryNode, NodeId, SolidOperation};
use ifc_geometry::compile::{
    compile_product_item_meshes, compile_product_item_meshes_with, compile_product_mesh,
    default_backend, ItemMesh, MeshClosure,
};
use ifc_geometry::error::GeometryError;
use ifc_model::{Codec, EntityId, Model};
use ifc_step::StepCodec;

const TOL: Tolerance = Tolerance::MILLIMETRE;

/// The base file: a window #10 whose Body is `items`, plus `extra` data.
fn window(items: &str, extra: &str) -> Model {
    let data = format!(
        "#1=IFCPROJECT('0YvctVUKr0kugbFTf53O9L',$,'P',$,$,$,$,(#2),#3);
#2=IFCGEOMETRICREPRESENTATIONCONTEXT($,'Model',3,1.E-05,#4,$);
#3=IFCUNITASSIGNMENT((#5));
#4=IFCAXIS2PLACEMENT3D(#6,$,$);
#5=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.);
#6=IFCCARTESIANPOINT((0.,0.,0.));
#7=IFCDIRECTION((0.,0.,1.));
#8=IFCAXIS2PLACEMENT2D(#9,$);
#9=IFCCARTESIANPOINT((0.,0.));
#10=IFCWINDOW('1YvctVUKr0kugbFTf53O9L',$,'Window',$,$,#11,#12,$,1.2,1.,.WINDOW.,.NOTDEFINED.,$);
#11=IFCLOCALPLACEMENT($,#4);
#12=IFCPRODUCTDEFINITIONSHAPE($,$,(#16));
#16=IFCSHAPEREPRESENTATION(#2,'Body','SweptSolid',({items}));
#30=IFCEXTRUDEDAREASOLID(#31,#4,#7,1.2);
#31=IFCRECTANGLEPROFILEDEF(.AREA.,$,#8,1.,0.1);
#40=IFCEXTRUDEDAREASOLID(#41,#42,#7,1.);
#41=IFCRECTANGLEPROFILEDEF(.AREA.,$,#8,0.8,0.01);
#42=IFCAXIS2PLACEMENT3D(#43,$,$);
#43=IFCCARTESIANPOINT((0.,0.5,0.1));
{extra}
"
    );
    parse(&data)
}

fn parse(data: &str) -> Model {
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
        .expect("fixture parses")
}

/// Two windows (#80 at x = 5, #90 at (10, 0, 0) with the map lifted 2 m)
/// sharing one map #70 whose representation lists the pane before the frame.
const MAPPED: &str = "#70=IFCREPRESENTATIONMAP(#4,#71);
#71=IFCSHAPEREPRESENTATION(#2,'Body','SweptSolid',(#140,#130));
#130=IFCEXTRUDEDAREASOLID(#31,#4,#7,1.2);
#140=IFCEXTRUDEDAREASOLID(#41,#42,#7,1.);
#80=IFCWINDOW('2Yvct0000000000000000A',$,'A',$,$,#81,#82,$,$,$,$,$,$);
#81=IFCLOCALPLACEMENT($,#83);
#83=IFCAXIS2PLACEMENT3D(#84,$,$);
#84=IFCCARTESIANPOINT((5.,0.,0.));
#82=IFCPRODUCTDEFINITIONSHAPE($,$,(#85));
#85=IFCSHAPEREPRESENTATION(#2,'Body','MappedRepresentation',(#86));
#86=IFCMAPPEDITEM(#70,#87);
#87=IFCCARTESIANTRANSFORMATIONOPERATOR3D($,$,#6,$,$);
#90=IFCWINDOW('3Yvct0000000000000000B',$,'B',$,$,#91,#92,$,$,$,$,$,$);
#91=IFCLOCALPLACEMENT($,#93);
#93=IFCAXIS2PLACEMENT3D(#94,$,$);
#94=IFCCARTESIANPOINT((10.,0.,0.));
#92=IFCPRODUCTDEFINITIONSHAPE($,$,(#95));
#95=IFCSHAPEREPRESENTATION(#2,'Body','MappedRepresentation',(#96));
#96=IFCMAPPEDITEM(#70,#97);
#97=IFCCARTESIANTRANSFORMATIONOPERATOR3D($,$,#98,$,$);
#98=IFCCARTESIANPOINT((0.,0.,2.));";

fn items_of(model: &Model, product: u64) -> Vec<ItemMesh> {
    compile_product_item_meshes(model, EntityId(product), TOL)
        .unwrap_or_else(|error| panic!("item meshes must compile, got {error}"))
        .expect("the product has a body")
}

fn product_mesh(model: &Model, product: u64) -> TriMesh {
    compile_product_mesh(model, EntityId(product), TOL)
        .expect("product compiles")
        .expect("the product has a body")
}

fn bounds(mesh: &TriMesh) -> ([f64; 3], [f64; 3]) {
    let b = mesh.bounds();
    ([b.min.x, b.min.y, b.min.z], [b.max.x, b.max.y, b.max.z])
}

fn joined(items: &[ItemMesh]) -> ([f64; 3], [f64; 3]) {
    let mut lo = [f64::INFINITY; 3];
    let mut hi = [f64::NEG_INFINITY; 3];
    for item in items {
        let (min, max) = bounds(&item.mesh);
        for axis in 0..3 {
            lo[axis] = lo[axis].min(min[axis]);
            hi[axis] = hi[axis].max(max[axis]);
        }
    }
    (lo, hi)
}

fn assert_near(actual: [f64; 3], expected: [f64; 3], what: &str) {
    for axis in 0..3 {
        assert!(
            (actual[axis] - expected[axis]).abs() < 1e-9,
            "{what}: expected {expected:?}, got {actual:?}"
        );
    }
}

fn shifted(point: [f64; 3], by: [f64; 3]) -> [f64; 3] {
    [point[0] + by[0], point[1] + by[1], point[2] + by[2]]
}

fn signed_volume(mesh: &TriMesh) -> f64 {
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

fn ids(items: &[ItemMesh]) -> Vec<EntityId> {
    items.iter().map(|item| item.item).collect()
}

#[test]
fn a_single_item_body_meshes_as_the_product() {
    let model = window("#30", "");
    let items = items_of(&model, 10);
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].item, EntityId(30));
    assert!(items[0].mapped_by.is_empty());
    assert_eq!(items[0].mesh, product_mesh(&model, 10));
    assert_eq!(items[0].closure, MeshClosure::Solid);
}

#[test]
fn a_clipped_single_item_meshes_as_the_product() {
    let model = window(
        "#50",
        "#50=IFCBOOLEANCLIPPINGRESULT(.DIFFERENCE.,#30,#51);
#51=IFCHALFSPACESOLID(#52,.F.);
#52=IFCPLANE(#53);
#53=IFCAXIS2PLACEMENT3D(#54,$,$);
#54=IFCCARTESIANPOINT((0.,0.,0.6));",
    );
    let items = items_of(&model, 10);
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].item, EntityId(50));
    assert_eq!(items[0].mesh, product_mesh(&model, 10));
}

/// The pane is listed first, so authored order puts #40 before #30.
#[test]
fn items_follow_authored_order_not_id_order() {
    let model = window("#40,#30", "");
    let items = items_of(&model, 10);
    assert_eq!(ids(&items), vec![EntityId(40), EntityId(30)]);
    assert!((signed_volume(&items[0].mesh) - 0.008).abs() < 1e-6);
    assert!((signed_volume(&items[1].mesh) - 0.12).abs() < 1e-6);

    let product = product_mesh(&model, 10);
    let triangles: usize = items.iter().map(|item| item.mesh.triangle_count()).sum();
    assert_eq!(triangles, product.triangle_count());
    let (lo, hi) = joined(&items);
    let (min, max) = bounds(&product);
    assert_near(lo, min, "joined minimum");
    assert_near(hi, max, "joined maximum");
}

#[test]
fn each_mapped_occurrence_gets_its_frame() {
    let model = window("#40,#30", MAPPED);
    let in_place = items_of(&model, 10);
    for (product, map_item, by) in [(80, 86, [5.0, 0.0, 0.0]), (90, 96, [10.0, 0.0, 2.0])] {
        let items = items_of(&model, product);
        assert_eq!(ids(&items), vec![EntityId(140), EntityId(130)]);
        for (item, plain) in items.iter().zip(&in_place) {
            assert_eq!(item.mapped_by, vec![EntityId(map_item)]);
            let (min, max) = bounds(&plain.mesh);
            let (got_min, got_max) = bounds(&item.mesh);
            assert_near(got_min, shifted(min, by), "leaf minimum");
            assert_near(got_max, shifted(max, by), "leaf maximum");
        }
        let whole = product_mesh(&model, product);
        let triangles: usize = items.iter().map(|item| item.mesh.triangle_count()).sum();
        assert_eq!(triangles, whole.triangle_count());
        let (lo, hi) = joined(&items);
        let (min, max) = bounds(&whole);
        assert_near(lo, min, "joined minimum");
        assert_near(hi, max, "joined maximum");
    }
}

/// The outer map rotates +90 degrees about Z, the inner moves +1 m along y.
/// Outer after inner puts the 0.2 m block at x = -1; the reverse at y = 1.
#[test]
fn nested_maps_compose_outer_after_inner() {
    let model = window(
        "#180",
        "#180=IFCMAPPEDITEM(#181,#182);
#181=IFCREPRESENTATIONMAP(#4,#183);
#182=IFCCARTESIANTRANSFORMATIONOPERATOR3D(#184,#185,#6,$,#7);
#183=IFCSHAPEREPRESENTATION(#2,'Body','MappedRepresentation',(#190));
#184=IFCDIRECTION((0.,1.,0.));
#185=IFCDIRECTION((-1.,0.,0.));
#190=IFCMAPPEDITEM(#191,#192);
#191=IFCREPRESENTATIONMAP(#4,#193);
#192=IFCCARTESIANTRANSFORMATIONOPERATOR3D($,$,#194,$,$);
#193=IFCSHAPEREPRESENTATION(#2,'Body','SweptSolid',(#150));
#194=IFCCARTESIANPOINT((0.,1.,0.));
#150=IFCEXTRUDEDAREASOLID(#151,#4,#7,1.);
#151=IFCRECTANGLEPROFILEDEF(.AREA.,$,#8,0.2,0.2);",
    );
    let items = items_of(&model, 10);
    assert_eq!(ids(&items), vec![EntityId(150)]);
    assert_eq!(items[0].mapped_by, vec![EntityId(180), EntityId(190)]);
    let (min, max) = bounds(&items[0].mesh);
    assert_near(min, [-1.1, -0.1, 0.0], "nested minimum");
    assert_near(max, [-0.9, 0.1, 1.0], "nested maximum");
    assert_eq!(items[0].mesh, product_mesh(&model, 10));
}

#[test]
fn no_body_is_none() {
    let model = window(
        "#30",
        "#60=IFCPRODUCTDEFINITIONSHAPE($,$,(#61));
#61=IFCSHAPEREPRESENTATION(#2,'Axis','Curve3D',(#62));
#62=IFCPOLYLINE((#6,#63));
#63=IFCCARTESIANPOINT((1.,0.,0.));
#64=IFCWINDOW('4Yvct0000000000000000C',$,'Axis',$,$,#11,#60,$,$,$,$,$,$);",
    );
    assert!(compile_product_item_meshes(&model, EntityId(64), TOL)
        .expect("no body is not an error")
        .is_none());
}

#[test]
fn an_empty_body_has_no_items() {
    let model = window("", "");
    let items = compile_product_item_meshes(&model, EntityId(10), TOL)
        .expect("an empty body is not an error")
        .expect("the product has a body representation");
    assert!(items.is_empty());
}

#[test]
fn a_dangling_item_refuses_the_call() {
    let model = window("#30,#99", "");
    let error = compile_product_item_meshes(&model, EntityId(10), TOL).expect_err("refused");
    let whole = compile_product_mesh(&model, EntityId(10), TOL).expect_err("refused");
    assert!(
        matches!(error, GeometryError::MissingEntity { missing, .. } if missing == EntityId(99)),
        "got {error:?}"
    );
    assert_eq!(error.to_string(), whole.to_string());
}

#[test]
fn a_cyclic_map_refuses() {
    let model = window(
        "#86",
        "#70=IFCREPRESENTATIONMAP(#4,#71);
#71=IFCSHAPEREPRESENTATION(#2,'Body','SweptSolid',(#30,#72));
#72=IFCMAPPEDITEM(#70,#87);
#86=IFCMAPPEDITEM(#70,#87);
#87=IFCCARTESIANTRANSFORMATIONOPERATOR3D($,$,#6,$,$);",
    );
    let error = compile_product_item_meshes(&model, EntityId(10), TOL).expect_err("refused");
    assert!(
        matches!(
            error,
            GeometryError::CyclicChain {
                kind: "mapped item",
                ..
            }
        ),
        "got {error:?}"
    );
}

/// Maps nested past the session's depth limit refuse, as the whole Body does.
#[test]
fn maps_nested_past_the_depth_limit_refuse() {
    // Each level enters its mapped item and its representation.
    let levels = 40;
    let mut extra = String::from("#87=IFCCARTESIANTRANSFORMATIONOPERATOR3D($,$,#6,$,$);\n");
    for level in 0..levels {
        let (item, map, rep) = (1000 + 3 * level, 1001 + 3 * level, 1002 + 3 * level);
        let inner = if level + 1 == levels {
            "#30".to_string()
        } else {
            format!("#{}", 1000 + 3 * (level + 1))
        };
        extra.push_str(&format!(
            "#{item}=IFCMAPPEDITEM(#{map},#87);
#{map}=IFCREPRESENTATIONMAP(#4,#{rep});
#{rep}=IFCSHAPEREPRESENTATION(#2,'Body','SweptSolid',({inner}));
"
        ));
    }
    let model = window("#1000", &extra);
    let error = compile_product_item_meshes(&model, EntityId(10), TOL).expect_err("refused");
    let whole = compile_product_mesh(&model, EntityId(10), TOL).expect_err("refused");
    assert!(
        matches!(
            error,
            GeometryError::ChainTooDeep {
                kind: "mapped item",
                ..
            }
        ),
        "got {error:?}"
    );
    assert_eq!(error.to_string(), whole.to_string());
}

/// The reference compiler, refusing any extrusion of depth 1 m (the pane).
#[derive(Debug)]
struct Proxy {
    refuse_pane: bool,
    calls: AtomicUsize,
}

impl Backend for Proxy {
    fn descriptor(&self) -> BackendDescriptor {
        default_backend().descriptor()
    }
}

fn is_pane(graph: &GeometryGraph, id: NodeId) -> bool {
    match graph.get(id) {
        Some(GeometryNode::SolidOperation(SolidOperation::Extrusion { depth, .. })) => {
            (depth - 1.0).abs() < 1e-9
        }
        Some(GeometryNode::Instance(instance)) => is_pane(graph, instance.source),
        _ => false,
    }
}

fn reaches_pane(graph: &GeometryGraph, root: NodeId) -> bool {
    let mut seen = vec![root];
    let mut index = 0;
    while index < seen.len() {
        if is_pane(graph, seen[index]) {
            return true;
        }
        if let Some(node) = graph.get(seen[index]) {
            for child in node.references() {
                if !seen.contains(&child) {
                    seen.push(child);
                }
            }
        }
        index += 1;
    }
    false
}

impl MeshCompiler for Proxy {
    fn compile_mesh(
        &self,
        graph: &GeometryGraph,
        root: NodeId,
        options: &ExecutionOptions,
    ) -> GeomResult<TriMesh> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if self.refuse_pane && reaches_pane(graph, root) {
            return Err(GeomError::InvalidInput("proxy refuses the pane".into()));
        }
        default_backend().compile_mesh(graph, root, options)
    }
}

#[test]
fn a_backend_refusal_names_the_product() {
    let model = window("#30,#40", "");
    let control = Proxy {
        refuse_pane: false,
        calls: AtomicUsize::new(0),
    };
    let items = compile_product_item_meshes_with(&control, &model, EntityId(10), TOL)
        .expect("the control compiles")
        .expect("body");
    assert_eq!(items.len(), 2);
    assert_eq!(control.calls.load(Ordering::SeqCst), 2);
    assert_eq!(ids(&items), vec![EntityId(30), EntityId(40)]);

    let refusing = Proxy {
        refuse_pane: true,
        calls: AtomicUsize::new(0),
    };
    let error = compile_product_item_meshes_with(&refusing, &model, EntityId(10), TOL)
        .expect_err("one refused leaf refuses the call");
    // The frame compiled first; the pane, last, refused it all.
    assert_eq!(refusing.calls.load(Ordering::SeqCst), 2);
    assert!(
        matches!(error, GeometryError::CompilationRefused { entity, .. } if entity == EntityId(10)),
        "got {error:?}"
    );
}
