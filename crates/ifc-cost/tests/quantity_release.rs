//! Cost quantity authoring is bound to the model's declared release and
//! writes the value bare (#190).
//!
//! From the EXPRESS sources: IFC2X3 TC1 `IfcQuantity<Kind>` declares `Name,
//! Description, Unit, <Kind>Value`; IFC4 ADD2 TC1 and IFC4X3 ADD2 add
//! `Formula`. Only IFC4X3 declares `IfcQuantityNumber`. `<Kind>Value` is
//! declared with a defined measure type, not a SELECT, in all three, so ISO
//! 10303-21 writes the value without a typed parameter.

use ifc_cost::mutation::{create_quantity, CostAuthoringError, QuantityDraft, QuantityKind};
use ifc_cost::quantity::CostQuantity;
use ifc_cost::SchemaVersion;
use ifc_model::{Codec, EntityId, Model, Transaction, Value};
use ifc_step::StepCodec;

const RELEASES: [(&str, SchemaVersion, usize); 3] = [
    ("IFC2X3", SchemaVersion::Ifc2x3, 4),
    ("IFC4", SchemaVersion::Ifc4, 5),
    ("IFC4X3_ADD2", SchemaVersion::Ifc4x3, 5),
];

fn declaring(schema: &str) -> Model {
    let mut model = Model::new();
    model.header_mut().schema = vec![schema.to_owned()];
    model
}

fn draft(kind: QuantityKind, value: f64) -> QuantityDraft<'static> {
    QuantityDraft::new(kind, "Measured", value)
}

fn refused(model: &Model, draft: QuantityDraft<'_>) -> CostAuthoringError {
    let mut tx = Transaction::new(model);
    let error = create_quantity(&mut tx, model, draft).expect_err("refused");
    assert!(tx.is_empty(), "a refusal staged {:?}", tx.edits());
    error
}

#[test]
fn a_quantity_has_its_release_layout_and_a_bare_value() {
    for (schema, _, arity) in RELEASES {
        let mut model = declaring(schema);
        let mut tx = Transaction::new(&model);
        let area = create_quantity(&mut tx, &model, draft(QuantityKind::Area, 12.5)).expect(schema);
        let count =
            create_quantity(&mut tx, &model, draft(QuantityKind::Count, 4.0)).expect(schema);
        tx.commit(&mut model).expect("commit");

        let bytes = StepCodec.write_bytes(&model).expect("written");
        let text = String::from_utf8(bytes.clone()).expect("utf8");
        assert!(!text.contains("MEASURE("), "{schema}: bare values:\n{text}");
        let back = StepCodec.read_bytes(&bytes).expect("read back");
        assert!(back.diagnostics().is_empty(), "{:?}", back.diagnostics());
        for (id, value) in [(area, Value::Real(12.5)), (count, Value::Integer(4))] {
            let record = back.get(id).expect("read back");
            assert_eq!(record.attributes.len(), arity, "{schema}: {record:?}");
            assert_eq!(record.attributes[3], value, "{schema}");
        }
        let view = CostQuantity::new(area, back.get(area).unwrap());
        assert_eq!(view.value(), Some(12.5), "{schema}");
    }
}

#[test]
fn what_the_release_cannot_hold_is_refused() {
    for (schema, version) in [
        ("IFC2X3", SchemaVersion::Ifc2x3),
        ("IFC4", SchemaVersion::Ifc4),
    ] {
        let model = declaring(schema);
        assert_eq!(
            refused(&model, draft(QuantityKind::Number, 1.0)),
            CostAuthoringError::EntityNotInSchema {
                entity: "IFCQUANTITYNUMBER",
                schema: version,
            }
        );
    }
    let ifc2x3 = declaring("IFC2X3");
    let formula = draft(QuantityKind::Area, 1.0).formula("l * h");
    assert_eq!(
        refused(&ifc2x3, formula),
        CostAuthoringError::AuthoringNotInSchema {
            entity: "IFCQUANTITYAREA",
            attribute: "Formula",
            schema: SchemaVersion::Ifc2x3,
        }
    );
    let mut several = Model::new();
    several.header_mut().schema = vec!["IFC4".to_owned(), "IFC2X3".to_owned()];
    assert_eq!(
        refused(&several, draft(QuantityKind::Length, 1.0)),
        CostAuthoringError::MultipleSchemas { schemas: 2 }
    );
    assert_eq!(
        refused(&declaring("IFC5"), draft(QuantityKind::Length, 1.0)),
        CostAuthoringError::UnsupportedSchema {
            schema: "IFC5".to_owned()
        }
    );
}

/// The reader accepts the bare form writers now emit and the typed form
/// files in the wild still carry.
#[test]
fn both_value_forms_read_back() {
    let text = "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
        FILE_NAME('','',(''),(''),'','','');\nFILE_SCHEMA(('IFC4'));\nENDSEC;\nDATA;\n\
        #1=IFCQUANTITYAREA('Bare',$,$,12.5,$);\n\
        #2=IFCQUANTITYAREA('Typed',$,$,IFCAREAMEASURE(12.5),$);\n\
        #3=IFCQUANTITYCOUNT('BareCount',$,$,4,$);\n\
        #4=IFCQUANTITYCOUNT('TypedCount',$,$,IFCCOUNTMEASURE(4),$);\n\
        ENDSEC;\nEND-ISO-10303-21;\n";
    let model = StepCodec.read_bytes(text.as_bytes()).expect("parses");
    for (id, expected) in [(1, 12.5), (2, 12.5), (3, 4.0), (4, 4.0)] {
        let id = EntityId(id);
        let view = CostQuantity::new(id, model.get(id).expect("present"));
        assert_eq!(view.value(), Some(expected), "#{}", id.0);
    }
}
