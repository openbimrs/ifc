//! #202: property templates in the declared release.
//!
//! From the EXPRESS sources: `IfcPropertySetTemplate`,
//! `IfcComplexPropertyTemplate` and `IfcRelDefinesByTemplate` are declared
//! by IFC4 ADD2 TC1 and IFC4X3 ADD2 with the same layout, and not by IFC2X3
//! TC1. The `*_with_owner_history` writers are read back through
//! `property_set_template`; `openbim-ifc`'s
//! `release_bound_owner_history_authoring` runs them through
//! `ifc-validate`.

mod predefined_support;

use ifc_model::{Codec, Edit, Entity, EntityId, Model, Transaction, Value};
use ifc_properties::{
    add_complex_property_template, add_complex_property_template_with_owner_history,
    add_property_set_template, add_property_set_template_with_owner_history, attach_template,
    attach_template_with_owner_history, property_set_template, PropertyError, SchemaVersion,
};
use ifc_step::StepCodec;
use predefined_support::{door_model, table};

const OWNER: EntityId = EntityId(44);
const G: [&str; 4] = [
    "0YvctVUKr0kugbFTf53O08",
    "0YvctVUKr0kugbFTf53O09",
    "0YvctVUKr0kugbFTf53O0A",
    "0YvctVUKr0kugbFTf53O0B",
];

/// A door model with an owner history `#44`, in `version`.
fn model(version: SchemaVersion) -> Model {
    door_model(
        version,
        &[],
        &[],
        &[
            "#40=IFCPERSON($,'Doe','Jane',$,$,$,$,$);".to_owned(),
            "#41=IFCORGANIZATION($,'Acme',$,$,$);".to_owned(),
            "#42=IFCPERSONANDORGANIZATION(#40,#41,$);".to_owned(),
            "#43=IFCAPPLICATION(#41,'1.0','Test','test');".to_owned(),
            "#44=IFCOWNERHISTORY(#42,#43,$,.NOCHANGE.,$,$,$,1700000000);".to_owned(),
        ],
    )
}

fn slot(version: SchemaVersion, entity: &str, attribute: &str) -> usize {
    table(version)
        .attribute_names(entity)
        .iter()
        .position(|name| name.eq_ignore_ascii_case(attribute))
        .unwrap_or_else(|| panic!("{entity}.{attribute}"))
}

/// Refused before anything is staged.
fn refused(
    model: &Model,
    author: impl Fn(&mut Transaction) -> Result<EntityId, PropertyError>,
) -> PropertyError {
    let mut tx = Transaction::new(model);
    let error = author(&mut tx).expect_err("refused");
    assert!(tx.is_empty(), "a refusal staged {:?}", tx.edits());
    error
}

/// IFC2X3 declares no template entity; each is refused, even with an
/// owner history.
#[test]
fn ifc2x3_declares_no_templates() {
    let model = model(SchemaVersion::Ifc2x3);

    let not_declared = |entity| PropertyError::EntityNotInSchema {
        entity,
        schema: SchemaVersion::Ifc2x3,
    };
    let member = EntityId(1);
    assert_eq!(
        refused(&model, |tx| add_property_set_template_with_owner_history(
            tx,
            &model,
            G[0],
            "Pset_T",
            None,
            &[member],
            OWNER
        )),
        not_declared("IFCPROPERTYSETTEMPLATE")
    );
    assert_eq!(
        refused(&model, |tx| {
            add_complex_property_template_with_owner_history(
                tx,
                &model,
                G[0],
                Some("C"),
                (None, Some("P_COMPLEX")),
                &[],
                OWNER,
            )
        }),
        not_declared("IFCCOMPLEXPROPERTYTEMPLATE")
    );
    assert_eq!(
        refused(&model, |tx| attach_template_with_owner_history(
            tx,
            &model,
            G[0],
            &[member],
            member,
            OWNER
        )),
        not_declared("IFCRELDEFINESBYTEMPLATE")
    );
}

/// IFC4 and IFC4X3 templates with an owner history, read back through
/// `property_set_template`; the writers that take no model are unchanged.
#[test]
fn templates_round_trip_in_ifc4_and_ifc4x3() {
    for version in [SchemaVersion::Ifc4, SchemaVersion::Ifc4x3] {
        let mut model = model(version);
        let mut tx = Transaction::new(&model);
        let names = table(version).attribute_names("IFCSIMPLEPROPERTYTEMPLATE");
        let mut simple = vec![Value::Null; names.len()];
        simple[0] = Value::Text("2YvctVUKr0kugbFTf53O08".into());
        simple[2] = Value::Text("Width".into());
        let simple = tx.create(Entity::new("IFCSIMPLEPROPERTYTEMPLATE", simple));
        let complex = add_complex_property_template_with_owner_history(
            &mut tx,
            &model,
            G[0],
            Some("Frame"),
            (Some("Frame"), Some("P_COMPLEX")),
            &[],
            OWNER,
        )
        .expect("complex");
        let set = add_property_set_template_with_owner_history(
            &mut tx,
            &model,
            G[1],
            "Pset_Custom",
            Some("IfcDoor"),
            &[simple, complex],
            OWNER,
        )
        .expect("set template");
        tx.commit(&mut model).expect("commit");
        let mut tx = Transaction::new(&model);
        let pset = tx.create(Entity::new(
            "IFCPROPERTYSET",
            vec![
                Value::Text(G[3].into()),
                Value::Null,
                Value::Text("Pset_Custom".into()),
                Value::Null,
                Value::List(vec![]),
            ],
        ));
        let rel = attach_template_with_owner_history(&mut tx, &model, G[2], &[pset], set, OWNER)
            .expect("rel");
        tx.commit(&mut model).expect("commit");

        let bytes = StepCodec.write_bytes(&model).expect("written");
        let back = StepCodec.read_bytes(&bytes).expect("read back");
        let template = property_set_template(&back, set).expect("read through the view");
        assert!(template.property("Width").is_some(), "{template:?}");
        for id in [complex, set, rel] {
            let record = back.get(id).unwrap();
            assert_eq!(
                record.attributes[slot(version, &record.type_name, "OwnerHistory")],
                Value::Ref(OWNER)
            );
        }

        // The writers without a model: the same records, `OwnerHistory` `$`.
        let mut plain = Transaction::new(&model);
        let id = add_property_set_template(
            &mut plain,
            G[1],
            "Pset_Custom",
            Some("IfcDoor"),
            &[simple, complex],
        )
        .unwrap();
        attach_template(&mut plain, G[2], &[pset], id).unwrap();
        add_complex_property_template(
            &mut plain,
            G[0],
            Some("Frame"),
            (Some("Frame"), Some("P_COMPLEX")),
            &[],
        )
        .unwrap();
        for edit in plain.edits() {
            let Edit::Create { entity, .. } = edit else {
                panic!("{edit:?}");
            };
            assert_eq!(entity.attributes[1], Value::Null);
            assert_eq!(
                entity.attributes.len(),
                table(version).attributes(&entity.type_name).len()
            );
        }
    }
}
