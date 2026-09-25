//! #51: reads resolve against the release `FILE_SCHEMA` declares.
//!
//! The IFC2X3 fixture is the one from openbimrs/ifc#51, built in memory with
//! an explicit header. Each IFC2X3 assertion has an IFC4 counterpart that
//! must stay exactly as it was in 0.2.0.

use ifc_classification::{
    classification_schema, ClassificationError, ClassificationSystem, ClassificationView,
    SchemaVersion,
};
use ifc_model::{Entity, EntityId, Model, Value};

fn text(value: &str) -> Value {
    Value::Text(value.into())
}
fn refs(ids: &[u64]) -> Value {
    Value::List(ids.iter().map(|id| Value::Ref(EntityId(*id))).collect())
}
fn put(model: &mut Model, id: u64, kind: &str, attributes: Vec<Value>) {
    model.insert(EntityId(id), Entity::new(kind, attributes));
}
fn declared(model: &mut Model, schemas: &[&str]) {
    model.header_mut().schema = schemas.iter().map(|s| (*s).to_string()).collect();
}

/// The fixture from #51, under an IFC2X3 header.
fn ifc2x3_fixture() -> Model {
    let mut m = Model::new();
    declared(&mut m, &["IFC2X3"]);
    put(
        &mut m,
        2,
        "IFCCALENDARDATE",
        vec![Value::Integer(9), Value::Integer(8), Value::Integer(2024)],
    );
    put(
        &mut m,
        3,
        "IFCCLASSIFICATION",
        vec![
            text("src"),
            text("2"),
            Value::Ref(EntityId(2)),
            text("Uniclass"),
        ],
    );
    put(
        &mut m,
        4,
        "IFCCLASSIFICATIONNOTATIONFACET",
        vec![text("Ss_25_10")],
    );
    put(&mut m, 5, "IFCCLASSIFICATIONNOTATION", vec![refs(&[4])]);
    put(
        &mut m,
        6,
        "IFCCLASSIFICATIONREFERENCE",
        vec![
            Value::Null,
            text("Pr_20"),
            Value::Null,
            Value::Ref(EntityId(3)),
        ],
    );
    for (id, name) in [(10, "W1"), (11, "W2")] {
        put(
            &mut m,
            id,
            "IFCWALL",
            vec![
                text("w"),
                Value::Ref(EntityId(1)),
                text(name),
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Null,
            ],
        );
    }
    put(
        &mut m,
        20,
        "IFCRELASSOCIATESCLASSIFICATION",
        vec![
            text("c1"),
            Value::Ref(EntityId(1)),
            Value::Null,
            Value::Null,
            refs(&[10]),
            Value::Ref(EntityId(5)),
        ],
    );
    put(
        &mut m,
        21,
        "IFCRELASSOCIATESCLASSIFICATION",
        vec![
            text("c2"),
            Value::Ref(EntityId(1)),
            Value::Null,
            Value::Null,
            refs(&[11]),
            Value::Ref(EntityId(6)),
        ],
    );
    m
}

/// The IFC4 equivalent: 7-slot classification with a string date.
fn ifc4_fixture() -> Model {
    let mut m = Model::new();
    declared(&mut m, &["IFC4"]);
    put(
        &mut m,
        3,
        "IFCCLASSIFICATION",
        vec![
            text("src"),
            text("2"),
            text("2024-08-09"),
            text("Uniclass"),
            text("desc"),
            Value::Null,
            Value::Null,
        ],
    );
    put(
        &mut m,
        6,
        "IFCCLASSIFICATIONREFERENCE",
        vec![
            Value::Null,
            text("Pr_20"),
            Value::Null,
            Value::Ref(EntityId(3)),
            Value::Null,
            Value::Null,
        ],
    );
    put(
        &mut m,
        11,
        "IFCWALL",
        vec![
            text("w"),
            Value::Null,
            text("W2"),
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Null,
        ],
    );
    put(
        &mut m,
        21,
        "IFCRELASSOCIATESCLASSIFICATION",
        vec![
            text("c2"),
            Value::Null,
            Value::Null,
            Value::Null,
            refs(&[11]),
            Value::Ref(EntityId(6)),
        ],
    );
    m
}

fn system(view: ClassificationView<'_>, id: u64) -> ClassificationSystem<'_> {
    view.systems()
        .find(|s| s.id() == EntityId(id))
        .expect("system present")
}

#[test]
fn the_bound_release_is_reported() {
    assert_eq!(
        classification_schema(&ifc2x3_fixture()),
        Ok(SchemaVersion::Ifc2x3)
    );
    assert_eq!(
        classification_schema(&ifc4_fixture()),
        Ok(SchemaVersion::Ifc4)
    );
    // An in-memory model with no header reads as IFC4, as in 0.2.0.
    assert_eq!(
        classification_schema(&Model::new()),
        Ok(SchemaVersion::Ifc4)
    );
    let mut mixed = Model::new();
    declared(&mut mixed, &["IFC2X3", "IFC4"]);
    assert_eq!(
        classification_schema(&mixed),
        Err(ClassificationError::MultipleSchemas { schemas: 2 })
    );
}

#[test]
fn an_ifc2x3_notation_is_a_valid_classification_source() {
    let model = ifc2x3_fixture();
    let view = ClassificationView::new(&model);
    // #10 is classified through an IfcClassificationNotation; IFC4's select
    // would reject it, IFC2X3's IfcClassificationNotationSelect accepts it.
    let effective = view.effective_classifications(EntityId(10)).unwrap();
    assert_eq!(effective.occurrence.len(), 1);
    let target = effective.occurrence[0]
        .relating_classification_id()
        .unwrap();
    assert_eq!(target, EntityId(5));
    assert_eq!(view.notation_values(target).unwrap(), vec!["Ss_25_10"]);
    // #11 through a reference, as before.
    let effective = view.effective_classifications(EntityId(11)).unwrap();
    assert_eq!(effective.occurrence.len(), 1);
}

#[test]
fn an_ifc2x3_calendar_date_is_structured_not_invalid() {
    let model = ifc2x3_fixture();
    let view = ClassificationView::new(&model);
    let uniclass = system(view, 3);
    assert_eq!(uniclass.name(), Ok("Uniclass"));
    assert_eq!(
        uniclass.edition_date(),
        Err(ClassificationError::StructuredValue {
            entity: "IFCCLASSIFICATION",
            id: EntityId(3),
            attribute: "EditionDate",
            target: EntityId(2),
        })
    );
    // IFC4: the same accessor returns the authored string, as in 0.2.0.
    let model = ifc4_fixture();
    assert_eq!(
        system(ClassificationView::new(&model), 3).edition_date(),
        Ok(Some("2024-08-09"))
    );
}

#[test]
fn attributes_the_release_lacks_are_not_in_schema_not_none() {
    let model = ifc2x3_fixture();
    let view = ClassificationView::new(&model);
    let uniclass = system(view, 3);
    for (result, attribute) in [
        (uniclass.description().map(|_| ()), "Description"),
        (uniclass.location().map(|_| ()), "Location"),
        (uniclass.reference_tokens().map(|_| ()), "ReferenceTokens"),
    ] {
        assert_eq!(
            result,
            Err(ClassificationError::NotInSchema {
                entity: "IFCCLASSIFICATION",
                id: EntityId(3),
                attribute,
                schema: SchemaVersion::Ifc2x3,
            })
        );
    }
    let reference = view.references().next().unwrap();
    // IFC2X3 ItemReference is read by the IFC4-named accessor.
    assert_eq!(reference.identification(), Ok(Some("Pr_20")));
    assert!(matches!(
        reference.sort(),
        Err(ClassificationError::NotInSchema {
            attribute: "Sort",
            ..
        })
    ));
    // IFC4 keeps its 0.2.0 answers.
    let model = ifc4_fixture();
    assert_eq!(
        system(ClassificationView::new(&model), 3).description(),
        Ok(Some("desc"))
    );
}

#[test]
fn a_wall_as_relating_classification_is_still_refused_on_ifc2x3() {
    let mut model = ifc2x3_fixture();
    put(
        &mut model,
        20,
        "IFCRELASSOCIATESCLASSIFICATION",
        vec![
            text("c1"),
            Value::Ref(EntityId(1)),
            Value::Null,
            Value::Null,
            refs(&[10]),
            Value::Ref(EntityId(11)),
        ],
    );
    let view = ClassificationView::new(&model);
    assert!(matches!(
        view.effective_classifications(EntityId(10)),
        Err(ClassificationError::ReferenceType {
            attribute: "RelatingClassification",
            expected: "IfcClassificationNotationSelect",
            ..
        })
    ));
}

#[test]
fn an_ifc4_classification_target_in_an_ifc2x3_file_is_refused() {
    // IFC4's select admits IfcClassification; IFC2X3's does not.
    let mut model = ifc2x3_fixture();
    put(
        &mut model,
        20,
        "IFCRELASSOCIATESCLASSIFICATION",
        vec![
            text("c1"),
            Value::Ref(EntityId(1)),
            Value::Null,
            Value::Null,
            refs(&[10]),
            Value::Ref(EntityId(3)),
        ],
    );
    let view = ClassificationView::new(&model);
    assert!(matches!(
        view.effective_classifications(EntityId(10)),
        Err(ClassificationError::ReferenceType {
            attribute: "RelatingClassification",
            ..
        })
    ));
}

#[test]
fn external_reference_relationships_are_not_in_ifc2x3() {
    let mut model = ifc2x3_fixture();
    put(
        &mut model,
        30,
        "IFCEXTERNALREFERENCERELATIONSHIP",
        vec![
            Value::Null,
            Value::Null,
            Value::Ref(EntityId(6)),
            refs(&[3]),
        ],
    );
    let view = ClassificationView::new(&model);
    assert!(matches!(
        view.external_reference_relationship(EntityId(30)),
        Err(ClassificationError::NotInSchema {
            schema: SchemaVersion::Ifc2x3,
            ..
        })
    ));
}

#[test]
fn notation_values_keep_facet_order() {
    // A notation's code is its facets' values in order; no separator is
    // invented and the order is the authored one.
    let mut model = ifc2x3_fixture();
    put(
        &mut model,
        7,
        "IFCCLASSIFICATIONNOTATIONFACET",
        vec![text("Ss")],
    );
    put(
        &mut model,
        8,
        "IFCCLASSIFICATIONNOTATIONFACET",
        vec![text("25")],
    );
    put(
        &mut model,
        9,
        "IFCCLASSIFICATIONNOTATIONFACET",
        vec![text("10")],
    );
    put(
        &mut model,
        12,
        "IFCCLASSIFICATIONNOTATION",
        vec![refs(&[7, 8, 9])],
    );
    let view = ClassificationView::new(&model);
    assert_eq!(
        view.notation_values(EntityId(12)).unwrap(),
        vec!["Ss", "25", "10"]
    );
    // A notation is IFC2X3-only.
    let mut ifc4 = ifc4_fixture();
    put(&mut ifc4, 12, "IFCCLASSIFICATIONNOTATION", vec![refs(&[3])]);
    assert!(matches!(
        ClassificationView::new(&ifc4).notation_values(EntityId(12)),
        Err(ClassificationError::NotInSchema {
            schema: SchemaVersion::Ifc4,
            ..
        })
    ));
}

#[test]
fn document_and_library_targets_follow_the_declared_select() {
    // `IfcRelAssociatesDocument.RelatingDocument` is `IfcDocumentSelect` in
    // both releases; a classification reference is not a member of it.
    let mut model = ifc2x3_fixture();
    put(
        &mut model,
        40,
        "IFCRELASSOCIATESDOCUMENT",
        vec![
            text("d1"),
            Value::Ref(EntityId(1)),
            Value::Null,
            Value::Null,
            refs(&[10]),
            Value::Ref(EntityId(6)),
        ],
    );
    put(
        &mut model,
        41,
        "IFCRELASSOCIATESLIBRARY",
        vec![
            text("l1"),
            Value::Ref(EntityId(1)),
            Value::Null,
            Value::Null,
            refs(&[10]),
            Value::Ref(EntityId(6)),
        ],
    );
    let view = ClassificationView::new(&model);
    assert!(matches!(
        view.document_assignments_for(EntityId(10)),
        Err(ClassificationError::ReferenceType {
            attribute: "RelatingDocument",
            expected: "IfcDocumentSelect",
            ..
        })
    ));
    assert!(matches!(
        view.library_assignments_for(EntityId(10)),
        Err(ClassificationError::ReferenceType {
            attribute: "RelatingLibrary",
            expected: "IfcLibrarySelect",
            ..
        })
    ));
}

#[test]
fn ifc4_external_reference_relationship_selects_come_from_the_ifc4_table() {
    // Read side (not authoring): an IFC4 relationship whose related resource
    // is a wall violates `IfcResourceObjectSelect`, and is refused on read.
    let mut model = ifc4_fixture();
    put(
        &mut model,
        30,
        "IFCEXTERNALREFERENCERELATIONSHIP",
        vec![
            Value::Null,
            Value::Null,
            Value::Ref(EntityId(6)),
            refs(&[11]),
        ],
    );
    let view = ClassificationView::new(&model);
    assert!(matches!(
        view.external_reference_relationship(EntityId(30)),
        Err(ClassificationError::ReferenceType {
            attribute: "RelatedResourceObjects",
            ..
        })
    ));
    // And a valid IFC4 one (a classification is a resource object) reads.
    put(
        &mut model,
        31,
        "IFCEXTERNALREFERENCERELATIONSHIP",
        vec![
            Value::Null,
            Value::Null,
            Value::Ref(EntityId(6)),
            refs(&[3]),
        ],
    );
    assert!(view_ok(&model, 31));
}

fn view_ok(model: &Model, id: u64) -> bool {
    ClassificationView::new(model)
        .external_reference_relationship(EntityId(id))
        .is_ok()
}
