//! Quantities and property sets authored in IFC2X3, IFC4 and IFC4X3
//! validate against their own release (#190, #191).
//!
//! Each release is authored through the public writers of `ifc-properties`
//! and `ifc-cost`, written to STEP, read back with `ifc-step`, and checked
//! by `ifc-validate` against the declared release's table. No record this
//! test wrote may carry an error finding. IFC2X3 requires
//! `IfcRoot.OwnerHistory`, so it is authored through the
//! `*_with_owner_history` writers; IFC4 and IFC4X3 through the writers that
//! leave it unset, which those releases allow.
//!
//! What the validator cannot see, a typed parameter in a non-SELECT slot,
//! is asserted on the STEP text directly: `ifc-validate` checks a typed
//! value against its wrapper, not against the slot's declared type.

#![cfg(all(
    feature = "validate",
    feature = "properties",
    feature = "cost",
    feature = "step"
))]

use ifc::cost::mutation::{create_quantity as cost_quantity, QuantityDraft, QuantityKind as Cost};
use ifc::properties::{
    add_element_quantity, add_element_quantity_with_owner_history, add_property_set,
    add_property_set_with_owner_history, add_property_single_value, attach_property_set,
    attach_property_set_with_owner_history, attach_type, attach_type_with_owner_history,
    create_quantity, QuantityKind, SchemaVersion,
};
use ifc::{Codec, Model, StepCodec, Value};
use ifc_model::{EntityId, Transaction};
use ifc_schema::for_version;

const OWNER: EntityId = EntityId(5);
const WALL: EntityId = EntityId(10);
const WALL_TYPE: EntityId = EntityId(11);

/// An owner history (`#5`), a wall (`#10`) and a wall type (`#11`), each
/// with the release's arity.
fn base(schema: &str, version: SchemaVersion) -> Model {
    let table = for_version(version).expect("bundled");
    let root = |id: u32, entity: &str, guid: &str, tail: &str| {
        let unset = ",$".repeat(table.attributes(entity).len() - 2 - tail.matches(',').count());
        format!("#{id}={entity}('{guid}',#5{unset}{tail});")
    };
    // IFC2X3 IfcWallType.PredefinedType is required; later releases keep it.
    let records = [
        "#1=IFCPERSON($,'Doe','Jane',$,$,$,$,$);".to_owned(),
        "#2=IFCORGANIZATION($,'Acme',$,$,$);".to_owned(),
        "#3=IFCPERSONANDORGANIZATION(#1,#2,$);".to_owned(),
        "#4=IFCAPPLICATION(#2,'1.0','Test','test');".to_owned(),
        "#5=IFCOWNERHISTORY(#3,#4,$,.NOCHANGE.,$,$,$,1700000000);".to_owned(),
        root(10, "IFCWALL", "1xS3BCk291UvhgP2dvNsgp", ""),
        root(11, "IFCWALLTYPE", "2xS3BCk291UvhgP2dvNsgp", ",.STANDARD."),
    ];
    let text = format!(
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
         FILE_NAME('','',(''),(''),'','','');\nFILE_SCHEMA(('{schema}'));\nENDSEC;\n\
         DATA;\n{}\nENDSEC;\nEND-ISO-10303-21;\n",
        records.join("\n")
    );
    StepCodec.read_bytes(text.as_bytes()).expect("parses")
}

/// Author through every writer this change touches; return the ids written.
fn author(model: &mut Model, owned: bool) -> Vec<EntityId> {
    let mut tx = Transaction::new(model);
    let property = add_property_single_value(
        &mut tx,
        "Width",
        None,
        Some(Value::Typed {
            type_name: "IFCLENGTHMEASURE".into(),
            value: Box::new(Value::Real(0.2)),
        }),
        None,
    )
    .expect("property");
    let area = create_quantity(&mut tx, model, QuantityKind::Area, "GrossArea", 12.5).expect("q");
    let count = create_quantity(&mut tx, model, QuantityKind::Count, "Doors", 4.0).expect("q");
    let draft = QuantityDraft {
        kind: Cost::Volume,
        name: "Concrete",
        description: None,
        unit: None,
        value: 3.75,
        formula: None,
    };
    let volume = cost_quantity(&mut tx, model, draft).expect("cost quantity");
    let (g1, g2) = ("0YvctVUKr0kugbFTf53O08", "0YvctVUKr0kugbFTf53O09");
    let pset = if owned {
        add_property_set_with_owner_history(
            &mut tx,
            model,
            g1,
            "Pset_T",
            None,
            &[("Width", property)],
            OWNER,
        )
    } else {
        add_property_set(&mut tx, g1, "Pset_T", None, &[("Width", property)])
    }
    .expect("pset");
    let quantities = [area, count, volume];
    let qto = if owned {
        add_element_quantity_with_owner_history(
            &mut tx,
            model,
            g2,
            "Qto_T",
            None,
            &quantities,
            OWNER,
        )
    } else {
        add_element_quantity(&mut tx, g2, "Qto_T", None, &quantities)
    }
    .expect("qto");
    tx.commit(model).expect("commit");

    let mut tx = Transaction::new(model);
    let guids = [
        "0YvctVUKr0kugbFTf53O0A",
        "0YvctVUKr0kugbFTf53O0B",
        "0YvctVUKr0kugbFTf53O0C",
    ];
    let rels = if owned {
        [
            attach_property_set_with_owner_history(&mut tx, model, guids[0], &[WALL], pset, OWNER),
            attach_property_set_with_owner_history(&mut tx, model, guids[1], &[WALL], qto, OWNER),
            attach_type_with_owner_history(&mut tx, model, guids[2], &[WALL], WALL_TYPE, OWNER),
        ]
    } else {
        [
            attach_property_set(&mut tx, model, guids[0], &[WALL], pset),
            attach_property_set(&mut tx, model, guids[1], &[WALL], qto),
            attach_type(&mut tx, model, guids[2], &[WALL], WALL_TYPE),
        ]
    }
    .map(|rel| rel.expect("attach"));
    tx.commit(model).expect("commit");

    let mut written = vec![property, area, count, volume, pset, qto];
    written.extend(rels);
    written
}

#[test]
fn authored_records_validate_in_their_release() {
    for (schema, version) in [
        ("IFC2X3", SchemaVersion::Ifc2x3),
        ("IFC4", SchemaVersion::Ifc4),
        ("IFC4X3_ADD2", SchemaVersion::Ifc4x3),
    ] {
        let mut model = base(schema, version);
        let written = author(&mut model, version == SchemaVersion::Ifc2x3);

        let bytes = StepCodec.write_bytes(&model).expect("written");
        let text = String::from_utf8(bytes.clone()).expect("utf8");
        for line in text.lines().filter(|line| line.contains("IFCQUANTITY")) {
            assert!(!line.contains("MEASURE("), "{schema}: bare value: {line}");
        }
        let back = StepCodec.read_bytes(&bytes).expect("read back");
        assert!(back.diagnostics().is_empty(), "{:?}", back.diagnostics());

        let report = ifc_validate::validate(&back, for_version(version).expect("bundled"));
        let errors: Vec<String> = report
            .findings()
            .iter()
            .filter(|finding| finding.severity == ifc_validate::Severity::Error)
            .filter(|finding| match &finding.path {
                ifc_validate::Path::Entity(id)
                | ifc_validate::Path::Attribute { entity: id, .. } => written.contains(id),
                ifc_validate::Path::File => false,
            })
            .map(|finding| format!("{} at {}: {}", finding.rule, finding.path, finding.message))
            .collect();
        assert!(errors.is_empty(), "{schema}:\n  {}", errors.join("\n  "));
    }
}

/// The oracle above is only trusted because it fails when it should: an
/// IFC2X3 relationship without its required `OwnerHistory`, written by
/// hand as the writers used to, is an error finding.
#[test]
fn the_validator_catches_an_ifc2x3_root_without_owner_history() {
    let mut model = base("IFC2X3", SchemaVersion::Ifc2x3);
    let mut tx = Transaction::new(&model);
    let rel = tx.create(ifc::Entity::new(
        "IFCRELDEFINESBYTYPE",
        vec![
            Value::Text("0YvctVUKr0kugbFTf53O0C".into()),
            Value::Null,
            Value::Null,
            Value::Null,
            Value::List(vec![Value::Ref(WALL)]),
            Value::Ref(WALL_TYPE),
        ],
    ));
    tx.commit(&mut model).expect("commit");
    let report = ifc_validate::validate(&model, for_version(SchemaVersion::Ifc2x3).unwrap());
    assert!(
        report.findings().iter().any(|finding| {
            finding.severity == ifc_validate::Severity::Error
                && matches!(
                    &finding.path,
                    ifc_validate::Path::Attribute { entity, index: 1, .. } if *entity == rel
                )
        }),
        "{:?}",
        report.findings()
    );
}
