//! Shared builders: inline IFC4X3 STEP files, lowering, the evaluator.

use std::path::PathBuf;

use axiolid_core::{Point3, Vec3};
use axiolid_model::{GeometryGraph, GeometryNode, NodeId};
use axiolid_reference::station::{station_section2, station_section3, SectionFrame};
use ifc_geometry::lower::{lower_representation_item, LoweredGeometry, LoweringSession};
use ifc_geometry::transform::Transform;
use ifc_geometry::{units, GeometryError};
use ifc_model::{Codec, EntityId, Model};
use ifc_step::StepCodec;

/// Coordinates agree to this, in metres.
pub const EPS: f64 = 1e-9;

/// An IFC4X3 file in metres (or millimetres) holding `records`.
///
/// Ids below 10 are taken by the project, context and units; `#9` is the
/// identity placement.
pub fn step(records: &str, millimetres: bool) -> Model {
    let prefix = if millimetres { ".MILLI." } else { "$" };
    let text = format!(
        "ISO-10303-21;
HEADER;
FILE_DESCRIPTION((''),'2;1');
FILE_NAME('','',(''),(''),'','','');
FILE_SCHEMA(('IFC4X3_ADD2'));
ENDSEC;
DATA;
#1=IFCPROJECT('0YvctVUKr0kugbFTf53O9L',$,'P',$,$,$,$,(#2),#3);
#2=IFCGEOMETRICREPRESENTATIONCONTEXT($,'Model',3,1.E-05,#4,$);
#3=IFCUNITASSIGNMENT((#5,#6));
#4=IFCAXIS2PLACEMENT3D(#7,$,$);
#5=IFCSIUNIT(*,.LENGTHUNIT.,{prefix},.METRE.);
#6=IFCSIUNIT(*,.PLANEANGLEUNIT.,$,.RADIAN.);
#7=IFCCARTESIANPOINT((0.,0.,0.));
#9=IFCAXIS2PLACEMENT3D(#7,$,$);
{records}
ENDSEC;
END-ISO-10303-21;
"
    );
    StepCodec
        .read_bytes(text.as_bytes())
        .unwrap_or_else(|e| panic!("the test file must parse: {e:?}\n{text}"))
}

/// The committed IFC4X3 geometry-family fixture (#243).
pub fn families() -> Model {
    fixture("synthetic_ifc4x3_geometry_families.ifc")
}

/// The committed alignment-layout fixture, with a linear placement.
pub fn layout() -> Model {
    fixture("synthetic_alignment_layout.ifc")
}

fn fixture(name: &str) -> Model {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../test/fixtures/synthetic-surfaces")
        .join(name);
    StepCodec
        .read_path(&path)
        .unwrap_or_else(|e| panic!("{}: {e:?}", path.display()))
}

/// The only entity of `kind` in `model`.
pub fn only(model: &Model, kind: &str) -> u64 {
    let ids = model.ids_of_type(kind);
    assert_eq!(ids.len(), 1, "expected one {kind}, found {}", ids.len());
    ids[0].0
}

/// Lower representation item `id` under the identity frame.
pub fn lower(model: &Model, id: u64) -> Result<LoweredGeometry, GeometryError> {
    let scale = units::resolve(model);
    let mut session = LoweringSession::new(model, &scale);
    let root = lower_representation_item(&mut session, EntityId(id), Transform::identity())?;
    Ok(session.finish(root).expect("a valid graph"))
}

/// The root node of a lowering.
pub fn root(lowered: &LoweredGeometry) -> &GeometryNode {
    lowered.graph.get(lowered.root).expect("the root exists")
}

/// The basis curve's section frame at `distance`, through the reference
/// evaluator (tests only: the library never evaluates).
pub fn section(graph: &GeometryGraph, basis: NodeId, distance: f64) -> SectionFrame {
    match graph.get(basis) {
        Some(GeometryNode::Curve3(curve)) => station_section3(curve, distance),
        Some(GeometryNode::Curve2(curve)) => station_section2(curve, distance),
        other => panic!("the basis is not an atomic curve: {other:?}"),
    }
    .unwrap_or_else(|e| panic!("the station resolves: {e:?}"))
}

/// Assert `error` is a typed refusal of `kind` naming `needle`.
pub fn refused(result: Result<LoweredGeometry, GeometryError>, unsupported: bool, needle: &str) {
    let error = result.map(|_| ()).expect_err(needle);
    assert_eq!(
        error.is_unsupported(),
        unsupported,
        "{needle}: wrong refusal kind: {error}"
    );
    assert!(error.to_string().contains(needle), "{needle}: {error}");
}

pub fn close_point(actual: Point3, expected: [f64; 3], what: &str) {
    let e = Point3::new(expected[0], expected[1], expected[2]);
    assert!(
        (actual - e).length() <= EPS,
        "{what}: {actual:?} != {expected:?}"
    );
}

pub fn close_vec(actual: Vec3, expected: [f64; 3], what: &str) {
    let e = Vec3::new(expected[0], expected[1], expected[2]);
    assert!(
        (actual - e).length() <= EPS,
        "{what}: {actual:?} != {expected:?}"
    );
}
