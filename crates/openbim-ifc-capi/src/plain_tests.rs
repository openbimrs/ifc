//! Boundary tests for plain-value writes (#342) and many objects' property
//! sets in one call (#358), called exactly as C would call them.

use openbim_ifc_binding_core::record::to_records;
use openbim_ifc_binding_core::value::Tagged;
use openbim_ifc_binding_core::{IfcModel, Record};

use crate::capability_tests::{last_code, parse, tape};
use crate::tape::{
    OPENBIM_IFC_KIND_EXACT, OPENBIM_IFC_KIND_INTEGER, OPENBIM_IFC_KIND_LIST, OPENBIM_IFC_KIND_REAL,
    OPENBIM_IFC_KIND_TEXT,
};
use crate::*;

const IFC4: &[u8] = b"ISO-10303-21;
HEADER;FILE_DESCRIPTION((''),'2;1');FILE_NAME('','',(''),(''),'','','');FILE_SCHEMA(('IFC4'));ENDSEC;
DATA;
#1=IFCWALL('0YvctVUKr0kugbFTf53O9L',$,'W',$,$,$,$,$,.NOTDEFINED.);
#4=IFCCURVESTYLE('c',$,$,$,$);
#7=IFCCARTESIANPOINT((0.,0.,0.));
ENDSEC;
END-ISO-10303-21;
";

fn node(kind: i32) -> OpenbimIfcValueNode {
    OpenbimIfcValueNode {
        kind,
        ..OpenbimIfcValueNode::default()
    }
}

/// A one-string plain tape.
fn text(value: &str) -> (Vec<OpenbimIfcValueNode>, Vec<u8>) {
    let mut n = node(OPENBIM_IFC_KIND_TEXT);
    n.str_len = value.len() as u64;
    (vec![n], value.as_bytes().to_vec())
}

fn write(
    model: OpenbimIfcModel,
    id: u64,
    name: &str,
    (nodes, strings): (Vec<OpenbimIfcValueNode>, Vec<u8>),
) -> OpenbimIfcStatus {
    // SAFETY: every buffer is valid for its length.
    unsafe {
        openbim_ifc_v0_1_entity_set_attribute_by_name_plain(
            model,
            id,
            name.as_ptr(),
            name.len(),
            nodes.as_ptr(),
            nodes.len(),
            strings.as_ptr(),
            strings.len(),
        )
    }
}

fn read(model: OpenbimIfcModel, id: u64, name: &str) -> Tagged {
    // SAFETY (closure): forwards the buffers the helper sized.
    tape(|n, nc, nr, s, sc, sr| unsafe {
        openbim_ifc_v0_1_entity_attribute_by_name(
            model,
            id,
            name.as_ptr(),
            name.len(),
            n,
            nc,
            nr,
            s,
            sc,
            sr,
        )
    })
    .unwrap()
}

#[test]
fn plain_values_are_coerced_against_the_declared_type() {
    let model = parse(IFC4);
    assert_eq!(write(model, 1, "Name", text("x")), OpenbimIfcStatus::Ok);
    assert_eq!(read(model, 1, "Name"), Tagged::Text("x".into()));
    assert_eq!(
        write(model, 1, "PredefinedType", text("standard")),
        OpenbimIfcStatus::Ok
    );
    assert_eq!(
        read(model, 1, "PredefinedType"),
        Tagged::Enum("STANDARD".into())
    );
    assert_eq!(
        write(model, 4, "CurveWidth", text("by layer")),
        OpenbimIfcStatus::Ok
    );
    assert_eq!(
        read(model, 4, "CurveWidth"),
        Tagged::Typed {
            type_name: "IFCDESCRIPTIVEMEASURE".into(),
            value: Box::new(Tagged::Text("by layer".into())),
        }
    );
    // A list of integers into LIST OF IfcLengthMeasure: reals.
    let mut list = node(OPENBIM_IFC_KIND_LIST);
    list.child_count = 2;
    let mut one = node(OPENBIM_IFC_KIND_INTEGER);
    one.int_value = 1;
    let mut two = node(OPENBIM_IFC_KIND_REAL);
    two.real_value = 2.5;
    assert_eq!(
        write(model, 7, "Coordinates", (vec![list, one, two], Vec::new())),
        OpenbimIfcStatus::Ok
    );
    assert_eq!(
        read(model, 7, "Coordinates"),
        Tagged::List(vec![Tagged::Real(1.0), Tagged::Real(2.5)])
    );
    // An EXACT node is written as given: text into an enumeration slot.
    let (mut nodes, strings) = text("odd");
    nodes.insert(0, node(OPENBIM_IFC_KIND_EXACT));
    assert_eq!(
        write(model, 1, "PredefinedType", (nodes, strings)),
        OpenbimIfcStatus::Ok
    );
    assert_eq!(read(model, 1, "PredefinedType"), Tagged::Text("odd".into()));

    let mut real = node(OPENBIM_IFC_KIND_REAL);
    real.real_value = 2.5;
    assert_eq!(
        write(model, 4, "CurveWidth", (vec![real], Vec::new())),
        OpenbimIfcStatus::AmbiguousValue
    );
    assert_eq!(last_code(model), "ambiguous-value");
    assert_eq!(
        write(model, 1, "PredefinedType", text("CURVED")),
        OpenbimIfcStatus::TypeMismatch
    );
    assert_eq!(last_code(model), "type-mismatch");
    // The EXACT kind is refused by an exact write.
    let exact = [node(OPENBIM_IFC_KIND_EXACT), node(OPENBIM_IFC_KIND_REAL)];
    // SAFETY: the buffers are valid for their lengths.
    let status = unsafe {
        openbim_ifc_v0_1_entity_set_attribute_by_name(
            model,
            1,
            "Name".as_ptr(),
            4,
            exact.as_ptr(),
            exact.len(),
            std::ptr::null(),
            0,
        )
    };
    assert_eq!(status, OpenbimIfcStatus::InvalidValue);
    assert_eq!(openbim_ifc_v0_1_model_destroy(model), OpenbimIfcStatus::Ok);
}

#[test]
fn many_objects_cross_as_the_core_records() {
    let bytes = std::fs::read(format!(
        "{}/../../test/fixtures/synthetic-properties/synthetic_properties.ifc",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap();
    let core = IfcModel::parse(&bytes).unwrap();
    let model = parse(&bytes);
    let list = |records: Vec<Record>| Tagged::List(records.iter().map(Record::to_tagged).collect());
    let mut count = usize::MAX;
    let count_out: *mut usize = &mut count;
    let ids = [30u64, 19, 9999];
    // SAFETY (closure): forwards the buffers the helper sized; `ids` and
    // `count` outlive every call.
    let some = tape(|n, nc, nr, s, sc, sr| unsafe {
        openbim_ifc_v0_1_model_property_sets_many(
            model,
            ids.as_ptr(),
            ids.len(),
            count_out,
            n,
            nc,
            nr,
            s,
            sc,
            sr,
        )
    })
    .unwrap();
    assert_eq!(count, 3);
    assert_eq!(
        some,
        list(to_records(&core.property_sets_many(Some(&ids)).unwrap()))
    );
    // SAFETY (closure): as above; null ids with count 0 select every
    // object definition.
    let every = tape(|n, nc, nr, s, sc, sr| unsafe {
        openbim_ifc_v0_1_model_property_sets_many(
            model,
            std::ptr::null(),
            0,
            count_out,
            n,
            nc,
            nr,
            s,
            sc,
            sr,
        )
    })
    .unwrap();
    assert_eq!(
        every,
        list(to_records(&core.property_sets_many(None).unwrap()))
    );
    // SAFETY: null ids with a non-zero count is refused before any read.
    let status = unsafe {
        openbim_ifc_v0_1_model_property_sets_many(
            model,
            std::ptr::null(),
            2,
            &mut count,
            std::ptr::null_mut(),
            0,
            &mut count,
            std::ptr::null_mut(),
            0,
            &mut count,
        )
    };
    assert_eq!(status, OpenbimIfcStatus::NullPointer);
    assert_eq!(openbim_ifc_v0_1_model_destroy(model), OpenbimIfcStatus::Ok);
}
