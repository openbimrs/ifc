//! A backend refusal during a net compile is attributed to the right entity.
//!
//! `opening_subtraction_compile.rs` covers refusals raised while LOWERING. A
//! refusal the BACKEND raises only arrives after the whole net graph was
//! handed over, as one error for one root, so `compile_product_mesh_net_with`
//! has to work out which piece was at fault. These tests plug in a proxy
//! kernel that delegates to the reference compiler but refuses one chosen
//! piece, and check the error names exactly that piece.
//!
//! Each proxy refuses a different node kind, so each test fails if the
//! attribution pass is removed, reordered, or blames the host by default.
#![cfg(feature = "compile-reference-backend")]

use std::sync::atomic::{AtomicUsize, Ordering};

use axiolid_contracts::{Backend, BackendDescriptor, ExecutionOptions, GeomError, GeomResult};
use axiolid_core::Tolerance;
use axiolid_mesh::TriMesh;
use axiolid_mesh_compile_contract::MeshCompiler;
use axiolid_model::{GeometryGraph, GeometryNode, NodeId, SolidOperation};
use ifc_geometry::compile::{compile_product_mesh_net_with, default_backend};
use ifc_geometry::error::GeometryError;
use ifc_model::{Codec, EntityId};
use ifc_step::StepCodec;

/// Which piece of the net graph the proxy refuses.
#[derive(Debug, Clone, Copy)]
enum Refuse {
    /// Any extrusion of this depth in metres: picks one solid by its shape.
    ExtrusionOfDepth(f64),
    /// Any boolean difference whose tool is an extrusion of this depth.
    SubtractionOfDepth(f64),
    /// Nothing: a control that must compile.
    Nothing,
}

/// The reference compiler, refusing one chosen piece of any graph.
#[derive(Debug)]
struct Proxy {
    refuse: Refuse,
    calls: AtomicUsize,
}

impl Proxy {
    fn new(refuse: Refuse) -> Self {
        Self {
            refuse,
            calls: AtomicUsize::new(0),
        }
    }
}

impl Backend for Proxy {
    fn descriptor(&self) -> BackendDescriptor {
        default_backend().descriptor()
    }
}

/// The extrusion depth a node carries, looking through the `Instance` that
/// lowering wraps around every placed extrusion.
fn extrusion_depth(graph: &GeometryGraph, id: NodeId) -> Option<f64> {
    match graph.get(id)? {
        GeometryNode::SolidOperation(SolidOperation::Extrusion { depth, .. }) => Some(*depth),
        GeometryNode::Instance(instance) => extrusion_depth(graph, instance.source),
        _ => None,
    }
}

/// Every node reachable from `root`, so a refusal is scoped to what the
/// backend was actually asked to compile.
fn reachable(graph: &GeometryGraph, root: NodeId) -> Vec<NodeId> {
    let mut seen = vec![root];
    let mut index = 0;
    while index < seen.len() {
        if let Some(node) = graph.get(seen[index]) {
            for child in node.references() {
                if !seen.contains(&child) {
                    seen.push(child);
                }
            }
        }
        index += 1;
    }
    seen
}

impl MeshCompiler for Proxy {
    fn compile_mesh(
        &self,
        graph: &GeometryGraph,
        root: NodeId,
        options: &ExecutionOptions,
    ) -> GeomResult<TriMesh> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let near = |a: f64, b: f64| (a - b).abs() < 1e-9;
        for id in reachable(graph, root) {
            let refused = match (self.refuse, graph.get(id)) {
                (Refuse::ExtrusionOfDepth(depth), _) => {
                    extrusion_depth(graph, id).is_some_and(|d| near(d, depth))
                }
                (
                    Refuse::SubtractionOfDepth(depth),
                    Some(GeometryNode::SolidOperation(SolidOperation::Boolean { right, .. })),
                ) => extrusion_depth(graph, *right).is_some_and(|d| near(d, depth)),
                _ => false,
            };
            if refused {
                return Err(GeomError::InvalidInput(format!("proxy refuses {id:?}")));
            }
        }
        default_backend().compile_mesh(graph, root, options)
    }
}

/// A wall (#10, 3 m high) voided by two openings of distinct depths:
/// #100 is 2.1 m high, #120 is 1.0 m high. Depth picks the piece to refuse.
fn fixture() -> ifc_model::Model {
    let mut data = String::from(
        "#1=IFCPROJECT('0YvctVUKr0kugbFTf53O9L',$,'P',$,$,$,$,(#2),#3);
#2=IFCGEOMETRICREPRESENTATIONCONTEXT($,'Model',3,1.E-05,#4,$);
#3=IFCUNITASSIGNMENT((#5));
#4=IFCAXIS2PLACEMENT3D(#6,$,$);
#5=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.);
#6=IFCCARTESIANPOINT((0.,0.,0.));
#7=IFCDIRECTION((0.,0.,1.));
#10=IFCWALL('1YvctVUKr0kugbFTf53O9L',$,'Wall',$,$,#11,#12,$,$);
#11=IFCLOCALPLACEMENT($,#4);
#12=IFCPRODUCTDEFINITIONSHAPE($,$,(#16));
#16=IFCSHAPEREPRESENTATION(#2,'Body','SweptSolid',(#17));
#17=IFCEXTRUDEDAREASOLID(#18,#4,#7,3.);
#18=IFCRECTANGLEPROFILEDEF(.AREA.,$,#19,4.,0.3);
#19=IFCAXIS2PLACEMENT2D(#20,$);
#20=IFCCARTESIANPOINT((0.,0.));
",
    );
    for (index, (x, height)) in [(-1.0, 2.1), (1.0, 1.0)].into_iter().enumerate() {
        let base = 100 + 20 * index;
        data.push_str(&format!(
            "#{o}=IFCOPENINGELEMENT('2YvctVUKr0kugbFTf53O{index:02}',$,'Opening',$,$,#{pl},#{sh},$,.OPENING.);
#{pl}=IFCLOCALPLACEMENT(#11,#{ax});
#{ax}=IFCAXIS2PLACEMENT3D(#{pt},$,$);
#{pt}=IFCCARTESIANPOINT(({x:?},0.,0.5));
#{sh}=IFCPRODUCTDEFINITIONSHAPE($,$,(#{rep}));
#{rep}=IFCSHAPEREPRESENTATION(#2,'Body','SweptSolid',(#{solid}));
#{solid}=IFCEXTRUDEDAREASOLID(#{profile},#4,#7,{height:?});
#{profile}=IFCRECTANGLEPROFILEDEF(.AREA.,$,#19,0.8,0.3);
#{rel}=IFCRELVOIDSELEMENT('3YvctVUKr0kugbFTf53O{index:02}',$,$,$,#10,#{o});
",
            o = base,
            pl = base + 1,
            ax = base + 2,
            pt = base + 3,
            sh = base + 4,
            rep = base + 5,
            solid = base + 6,
            profile = base + 7,
            rel = base + 8,
        ));
    }
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

fn run(refuse: Refuse) -> (GeometryResult, usize) {
    let proxy = Proxy::new(refuse);
    let model = fixture();
    let result = compile_product_mesh_net_with(&proxy, &model, EntityId(10), Tolerance::MILLIMETRE)
        .map(|net| net.map(|net| net.openings));
    (result, proxy.calls.load(Ordering::SeqCst))
}

type GeometryResult = Result<Option<Vec<EntityId>>, GeometryError>;

/// The blamed opening, failing if the error is anything else.
fn blamed(result: GeometryResult) -> EntityId {
    match result {
        Err(GeometryError::OpeningNotSubtracted {
            host,
            opening,
            cause,
        }) => {
            assert_eq!(host, EntityId(10));
            assert!(
                matches!(*cause, GeometryError::CompilationRefused { .. }),
                "cause: {cause:?}"
            );
            opening
        }
        other => panic!("expected OpeningNotSubtracted, got {other:?}"),
    }
}

/// The control: the proxy compiles when it refuses nothing, in ONE call.
#[test]
fn the_success_path_compiles_the_net_graph_once() {
    let (result, calls) = run(Refuse::Nothing);
    assert_eq!(
        result.expect("compiles"),
        Some(vec![EntityId(100), EntityId(120)])
    );
    assert_eq!(
        calls, 1,
        "a successful net compile must not re-compile pieces"
    );
}

/// A second opening's body the kernel cannot mesh is blamed, not the first.
#[test]
fn an_opening_body_the_backend_refuses_is_named() {
    let (result, _) = run(Refuse::ExtrusionOfDepth(1.0));
    assert_eq!(blamed(result), EntityId(120));
}

/// A subtraction the kernel refuses, with both operands fine, is named.
#[test]
fn a_subtraction_the_backend_refuses_is_named() {
    let (result, _) = run(Refuse::SubtractionOfDepth(2.1));
    assert_eq!(blamed(result), EntityId(100));
}

/// A host whose own body the backend refuses is NOT blamed on an opening.
///
/// Every step would fail, so the first opening would otherwise take the
/// blame for a wall that cannot be meshed even gross.
#[test]
fn a_refused_host_body_blames_the_host() {
    let (result, _) = run(Refuse::ExtrusionOfDepth(3.0));
    match result {
        Err(GeometryError::CompilationRefused { entity, .. }) => {
            assert_eq!(entity, EntityId(10));
        }
        other => panic!("expected CompilationRefused on the host, got {other:?}"),
    }
}
