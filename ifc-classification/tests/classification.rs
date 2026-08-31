use ifc_classification::{
    associate_classification, associate_document, associate_library, create_classification,
    create_classification_reference, create_document, create_document_reference, create_library,
    create_library_reference, AssociationDraft, ClassificationDraft, ClassificationError,
    ClassificationReferenceDraft, ClassificationView, DocumentDraft, DocumentReferenceDraft,
    LibraryDraft, LibraryReferenceDraft,
};
use ifc_model::{Budget, Entity, EntityId, Model, Transaction, Value};

fn text(value: &str) -> Value {
    Value::Text(value.into())
}
fn refs(ids: &[EntityId]) -> Value {
    Value::List(ids.iter().copied().map(Value::Ref).collect())
}
fn gid(seed: u8) -> String {
    ifc_model::guid::Guid::from_uuid([seed; 16]).to_string()
}

#[test]
fn bundled_schema_pins_every_interpreted_ifc4_slot() {
    let schema = ifc_schema::ifc4();
    assert_eq!(
        schema.attribute_names("IFCCLASSIFICATION"),
        [
            "Source",
            "Edition",
            "EditionDate",
            "Name",
            "Description",
            "Location",
            "ReferenceTokens"
        ]
    );
    assert_eq!(
        schema.attribute_names("IFCCLASSIFICATIONREFERENCE"),
        [
            "Location",
            "Identification",
            "Name",
            "ReferencedSource",
            "Description",
            "Sort"
        ]
    );
    assert_eq!(
        schema.attribute_names("IFCDOCUMENTREFERENCE"),
        [
            "Location",
            "Identification",
            "Name",
            "Description",
            "ReferencedDocument"
        ]
    );
    assert_eq!(
        schema.attribute_names("IFCLIBRARYREFERENCE"),
        [
            "Location",
            "Identification",
            "Name",
            "Description",
            "Language",
            "ReferencedLibrary"
        ]
    );
    for relation in [
        "IFCRELASSOCIATESCLASSIFICATION",
        "IFCRELASSOCIATESDOCUMENT",
        "IFCRELASSOCIATESLIBRARY",
    ] {
        assert_eq!(
            &schema.attribute_names(relation)[..5],
            [
                "GlobalId",
                "OwnerHistory",
                "Name",
                "Description",
                "RelatedObjects"
            ]
        );
        assert_eq!(schema.attribute_names(relation).len(), 6);
    }
}

fn representative_model() -> (Model, [EntityId; 14]) {
    let mut model = Model::new();
    let system = model.push(Entity::new(
        "IFCCLASSIFICATION",
        vec![
            text("NBS"),
            text("2025"),
            text("2025-01-01"),
            text("Uniclass"),
            text("System"),
            text("https://example/class"),
            Value::List(vec![text("Co"), text("Ss")]),
        ],
    ));
    let root = model.push(Entity::new(
        "IFCCLASSIFICATIONREFERENCE",
        vec![
            text("https://example/co"),
            text("Co"),
            text("Complexes"),
            Value::Ref(system),
            text("Root facet"),
            text("01"),
        ],
    ));
    let child = model.push(Entity::new(
        "IFCCLASSIFICATIONREFERENCE",
        vec![
            Value::Null,
            text("Co_20"),
            text("Administrative"),
            Value::Ref(root),
            Value::Null,
            text("02"),
        ],
    ));
    let owner = model.push(Entity::new("IFCPERSON", vec![]));
    let document = model.push(Entity::new(
        "IFCDOCUMENTINFORMATION",
        vec![
            text("DOC-1"),
            text("Specification"),
            text("Description"),
            text("https://example/spec"),
            text("Purpose"),
            text("Construction"),
            text("Project"),
            text("B"),
            Value::Ref(owner),
            refs(&[owner]),
            text("2025-01-02T03:04:05"),
            text("2025-02-03T04:05:06"),
            text("application/pdf"),
            text("2025-01-01"),
            text("2026-01-01"),
            Value::Enum("PUBLIC".into()),
            Value::Enum("FINAL".into()),
        ],
    ));
    let document_ref = model.push(Entity::new(
        "IFCDOCUMENTREFERENCE",
        vec![
            text("https://example/spec#7"),
            text("7"),
            Value::Null,
            text("Clause"),
            Value::Ref(document),
        ],
    ));
    let library = model.push(Entity::new(
        "IFCLIBRARYINFORMATION",
        vec![
            text("Product library"),
            text("2"),
            Value::Ref(owner),
            text("2025-01-01T00:00:00"),
            text("https://example/lib"),
            text("Products"),
        ],
    ));
    let library_ref = model.push(Entity::new(
        "IFCLIBRARYREFERENCE",
        vec![
            text("https://example/lib/p1"),
            text("P1"),
            text("Pump"),
            text("Entry"),
            text("en"),
            Value::Ref(library),
        ],
    ));
    let wall = model.push(Entity::new("IFCWALL", vec![]));
    let wall_type = model.push(Entity::new("IFCWALLTYPE", vec![]));
    let class_rel = model.push(Entity::new(
        "IFCRELASSOCIATESCLASSIFICATION",
        vec![
            text(&gid(1)),
            Value::Null,
            Value::Null,
            Value::Null,
            refs(&[wall]),
            Value::Ref(child),
        ],
    ));
    let doc_rel = model.push(Entity::new(
        "IFCRELASSOCIATESDOCUMENT",
        vec![
            text(&gid(2)),
            Value::Null,
            Value::Null,
            Value::Null,
            refs(&[wall]),
            Value::Ref(document_ref),
        ],
    ));
    let lib_rel = model.push(Entity::new(
        "IFCRELASSOCIATESLIBRARY",
        vec![
            text(&gid(3)),
            Value::Null,
            Value::Null,
            Value::Null,
            refs(&[wall]),
            Value::Ref(library_ref),
        ],
    ));
    let type_rel = model.push(Entity::new(
        "IFCRELDEFINESBYTYPE",
        vec![
            text(&gid(4)),
            Value::Null,
            Value::Null,
            Value::Null,
            refs(&[wall]),
            Value::Ref(wall_type),
        ],
    ));
    let type_class_rel = model.push(Entity::new(
        "IFCRELASSOCIATESCLASSIFICATION",
        vec![
            text(&gid(5)),
            Value::Null,
            Value::Null,
            Value::Null,
            refs(&[wall_type]),
            Value::Ref(root),
        ],
    ));
    (
        model,
        [
            system,
            root,
            child,
            owner,
            document,
            document_ref,
            library,
            library_ref,
            wall,
            wall_type,
            class_rel,
            doc_rel,
            lib_rel,
            type_rel.max(type_class_rel),
        ],
    )
}

#[test]
fn borrowed_views_decode_information_references_and_assignments() {
    let (model, ids) = representative_model();
    let view = ClassificationView::new(&model);
    let system = view.systems().next().unwrap();
    assert_eq!(system.id(), ids[0]);
    assert_eq!(system.name().unwrap(), "Uniclass");
    assert_eq!(system.reference_tokens().unwrap().unwrap(), ["Co", "Ss"]);
    let child = view.references().find(|v| v.id() == ids[2]).unwrap();
    assert_eq!(child.identification().unwrap(), Some("Co_20"));
    assert_eq!(child.referenced_source_id().unwrap(), Some(ids[1]));
    let document = view.documents().next().unwrap();
    assert_eq!(document.identification().unwrap(), "DOC-1");
    assert_eq!(document.editors().unwrap().unwrap(), [ids[3]]);
    assert_eq!(document.confidentiality().unwrap(), Some("PUBLIC"));
    let document_ref = view.document_references().next().unwrap();
    assert_eq!(document_ref.referenced_document_id().unwrap(), Some(ids[4]));
    document_ref.validate().unwrap();
    let library = view.libraries().next().unwrap();
    assert_eq!(library.name().unwrap(), "Product library");
    let library_ref = view.library_references().next().unwrap();
    assert_eq!(library_ref.language().unwrap(), Some("en"));
    assert_eq!(library_ref.referenced_library_id().unwrap(), Some(ids[6]));
    assert_eq!(
        view.classification_assignments_for(ids[8]).unwrap().len(),
        1
    );
    assert_eq!(view.document_assignments_for(ids[8]).unwrap().len(), 1);
    assert_eq!(view.library_assignments_for(ids[8]).unwrap().len(), 1);
    let unknown = EntityId(99_999);
    assert!(matches!(
        view.classification_assignments_for(unknown),
        Err(ClassificationError::UnknownEntity { id }) if id == unknown
    ));
    assert!(matches!(
        view.document_assignments_for(unknown),
        Err(ClassificationError::UnknownEntity { id }) if id == unknown
    ));
    assert!(matches!(
        view.library_assignments_for(unknown),
        Err(ClassificationError::UnknownEntity { id }) if id == unknown
    ));
}

#[test]
fn hierarchy_and_occurrence_type_sources_are_explicit() {
    let (mut model, ids) = representative_model();
    let view = ClassificationView::new(&model);
    let hierarchy = view.hierarchy_from(ids[2], Budget::default()).unwrap();
    assert_eq!(
        hierarchy
            .references
            .iter()
            .map(|v| v.id())
            .collect::<Vec<_>>(),
        [ids[2], ids[1]]
    );
    assert_eq!(hierarchy.system.unwrap().id(), ids[0]);
    assert_eq!(
        view.children_of(ids[1])
            .unwrap()
            .iter()
            .map(|v| v.id())
            .collect::<Vec<_>>(),
        [ids[2]]
    );
    let effective = view.effective_classifications(ids[8]).unwrap();
    assert_eq!(effective.occurrence.len(), 1);
    assert_eq!(effective.type_object, Some(ids[9]));
    assert_eq!(effective.inherited.len(), 1);
    assert_ne!(
        effective.occurrence[0]
            .relating_classification_id()
            .unwrap(),
        effective.inherited[0].relating_classification_id().unwrap()
    );
    model.push(Entity::new(
        "IFCRELDEFINESBYTYPE",
        vec![
            text(&gid(5)),
            Value::Null,
            Value::Null,
            Value::Null,
            Value::List(vec![Value::Ref(ids[8])]),
            Value::Ref(ids[9]),
        ],
    ));
    assert!(matches!(
        ClassificationView::new(&model).effective_classifications(ids[8]),
        Err(ClassificationError::AmbiguousType { count: 2, .. })
    ));
}

#[test]
fn malformed_hierarchies_and_values_fail_loudly() {
    let mut model = Model::new();
    let a = model.push(Entity::new(
        "IFCCLASSIFICATIONREFERENCE",
        vec![
            Value::Null,
            text("A"),
            Value::Null,
            Value::Ref(EntityId(2)),
            Value::Null,
            Value::Null,
        ],
    ));
    let b = model.push(Entity::new(
        "IFCCLASSIFICATIONREFERENCE",
        vec![
            Value::Null,
            text("B"),
            Value::Null,
            Value::Ref(a),
            Value::Null,
            Value::Null,
        ],
    ));
    assert_eq!(b, EntityId(2));
    let view = ClassificationView::new(&model);
    assert!(matches!(
        view.hierarchy_from(a, Budget::default()),
        Err(ClassificationError::Cycle { .. })
    ));
    assert!(matches!(
        view.hierarchy_from(
            a,
            Budget {
                max_depth: 0,
                max_nodes: 10
            }
        ),
        Err(ClassificationError::BudgetExceeded { .. })
    ));
    let wall = model.push(Entity::new("IFCWALL", vec![]));
    let wrong = model.push(Entity::new(
        "IFCCLASSIFICATIONREFERENCE",
        vec![
            Value::Null,
            text("W"),
            Value::Null,
            Value::Ref(wall),
            Value::Null,
            Value::Null,
        ],
    ));
    let dangling = model.push(Entity::new(
        "IFCCLASSIFICATIONREFERENCE",
        vec![
            Value::Null,
            text("D"),
            Value::Null,
            Value::Ref(EntityId(999)),
            Value::Null,
            Value::Null,
        ],
    ));
    let view = ClassificationView::new(&model);
    assert!(matches!(
        view.hierarchy_from(wrong, Budget::default()),
        Err(ClassificationError::ReferenceType { .. })
    ));
    assert!(matches!(
        view.hierarchy_from(dangling, Budget::default()),
        Err(ClassificationError::DanglingReference { .. })
    ));
    let bad_doc = model.push(Entity::new(
        "IFCDOCUMENTREFERENCE",
        vec![
            Value::Null,
            Value::Null,
            text("Name"),
            Value::Null,
            Value::Ref(EntityId(99)),
        ],
    ));
    let bad = ifc_classification::DocumentReference::try_new(bad_doc, model.get(bad_doc).unwrap())
        .unwrap();
    assert!(
        bad.validate().is_err(),
        "WR1 requires exactly one of Name or ReferencedDocument"
    );
    let missing_external_identity = model.push(Entity::new(
        "IFCDOCUMENTREFERENCE",
        vec![
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Ref(a),
        ],
    ));
    let missing = ifc_classification::DocumentReference::try_new(
        missing_external_identity,
        model.get(missing_external_identity).unwrap(),
    )
    .unwrap();
    assert!(matches!(
        missing.validate(),
        Err(ClassificationError::InvalidValue {
            attribute: "IfcExternalReference.WR1",
            ..
        })
    ));
    let point = model.push(Entity::new("IFCCARTESIANPOINT", vec![Value::List(vec![])]));
    let system = model.push(Entity::new(
        "IFCCLASSIFICATION",
        vec![
            Value::Null,
            Value::Null,
            Value::Null,
            text("System"),
            Value::Null,
            Value::Null,
            Value::Null,
        ],
    ));
    model.push(Entity::new(
        "IFCRELASSOCIATESCLASSIFICATION",
        vec![
            text("relationship"),
            Value::Null,
            Value::Null,
            Value::Null,
            refs(&[point]),
            Value::Ref(system),
        ],
    ));
    assert!(matches!(
        ClassificationView::new(&model).classification_assignments_for(point),
        Err(ClassificationError::ReferenceType {
            expected: "IfcDefinitionSelect",
            ..
        })
    ));
    model.push(Entity::new(
        "IFCRELASSOCIATESCLASSIFICATION",
        vec![
            text("duplicate relationship"),
            Value::Null,
            Value::Null,
            Value::Null,
            refs(&[wall, wall]),
            Value::Ref(system),
        ],
    ));
    assert!(matches!(
        ClassificationView::new(&model).classification_assignments_for(wall),
        Err(ClassificationError::InvalidValue {
            attribute: "RelatedObjects",
            ..
        })
    ));
}

#[test]
fn transactional_authoring_roundtrips_all_owned_records() {
    let mut model = Model::new();
    let wall = model.push(Entity::new("IFCWALL", vec![]));
    let mut tx = Transaction::new(&model);
    let system = create_classification(
        &mut tx,
        ClassificationDraft {
            source: Some("NBS"),
            edition: Some("2025"),
            edition_date: None,
            name: "Uniclass",
            description: None,
            location: None,
            reference_tokens: Some(&["Co"]),
        },
    )
    .unwrap();
    let class_ref = create_classification_reference(
        &mut tx,
        &model,
        ClassificationReferenceDraft {
            location: None,
            identification: Some("Co_20"),
            name: Some("Administrative"),
            referenced_source: Some(system),
            description: None,
            sort: None,
        },
    )
    .unwrap();
    let document = create_document(
        &mut tx,
        &model,
        DocumentDraft {
            identification: "DOC-1",
            name: "Spec",
            description: None,
            location: None,
            purpose: None,
            intended_use: None,
            scope: None,
            revision: None,
            document_owner: None,
            editors: None,
            creation_time: None,
            last_revision_time: None,
            electronic_format: None,
            valid_from: None,
            valid_until: None,
            confidentiality: None,
            status: None,
        },
    )
    .unwrap();
    let document_ref = create_document_reference(
        &mut tx,
        &model,
        DocumentReferenceDraft {
            location: None,
            identification: Some("7"),
            name: None,
            description: None,
            referenced_document: Some(document),
        },
    )
    .unwrap();
    let library = create_library(
        &mut tx,
        &model,
        LibraryDraft {
            name: "Products",
            version: None,
            publisher: None,
            version_date: None,
            location: None,
            description: None,
        },
    )
    .unwrap();
    let library_ref = create_library_reference(
        &mut tx,
        &model,
        LibraryReferenceDraft {
            location: None,
            identification: Some("P1"),
            name: None,
            description: None,
            language: Some("en"),
            referenced_library: Some(library),
        },
    )
    .unwrap();
    let related_objects = [wall];
    let relation = |seed| AssociationDraft {
        global_id: Box::leak(gid(seed).into_boxed_str()),
        name: None,
        description: None,
        related_objects: &related_objects,
    };
    associate_classification(&mut tx, &model, relation(10), class_ref).unwrap();
    associate_document(&mut tx, &model, relation(11), document_ref).unwrap();
    associate_library(&mut tx, &model, relation(12), library_ref).unwrap();
    tx.commit(&mut model).unwrap();
    let view = ClassificationView::new(&model);
    assert_eq!(view.systems().count(), 1);
    assert_eq!(view.references().count(), 1);
    assert_eq!(view.documents().count(), 1);
    assert_eq!(view.document_references().count(), 1);
    assert_eq!(view.libraries().count(), 1);
    assert_eq!(view.library_references().count(), 1);
    assert_eq!(view.classification_assignments_for(wall).unwrap().len(), 1);
}

#[test]
fn invalid_authoring_stages_nothing_and_failed_commit_rolls_back() {
    let mut model = Model::new();
    let wall = model.push(Entity::new("IFCWALL", vec![]));
    let point = model.push(Entity::new("IFCCARTESIANPOINT", vec![Value::List(vec![])]));
    let mut document_attrs = vec![text("DOC"), text("Document")];
    document_attrs.resize(17, Value::Null);
    let document = model.push(Entity::new("IFCDOCUMENTINFORMATION", document_attrs));
    let mut tx = Transaction::new(&model);
    let invalid = create_document_reference(
        &mut tx,
        &model,
        DocumentReferenceDraft {
            location: None,
            identification: None,
            name: Some("Name"),
            description: None,
            referenced_document: Some(document),
        },
    );
    assert!(invalid.is_err());
    assert!(tx.is_empty());
    assert!(create_document_reference(
        &mut tx,
        &model,
        DocumentReferenceDraft {
            location: None,
            identification: None,
            name: None,
            description: None,
            referenced_document: Some(document),
        },
    )
    .is_err());
    assert!(tx.is_empty());
    assert!(create_classification_reference(
        &mut tx,
        &model,
        ClassificationReferenceDraft {
            location: None,
            identification: None,
            name: None,
            referenced_source: None,
            description: None,
            sort: None,
        },
    )
    .is_err());
    assert!(tx.is_empty());
    assert!(create_library(
        &mut tx,
        &model,
        LibraryDraft {
            name: "Invalid publisher",
            version: None,
            publisher: Some(wall),
            version_date: None,
            location: None,
            description: None,
        },
    )
    .is_err());
    assert!(tx.is_empty());
    let system = create_classification(
        &mut tx,
        ClassificationDraft {
            source: None,
            edition: None,
            edition_date: None,
            name: "System",
            description: None,
            location: None,
            reference_tokens: None,
        },
    )
    .unwrap();
    let staged = tx.len();
    assert!(associate_classification(
        &mut tx,
        &model,
        AssociationDraft {
            global_id: "not-an-ifc-guid",
            name: None,
            description: None,
            related_objects: &[wall],
        },
        system,
    )
    .is_err());
    assert_eq!(tx.len(), staged);
    assert!(associate_classification(
        &mut tx,
        &model,
        AssociationDraft {
            global_id: &gid(18),
            name: None,
            description: None,
            related_objects: &[point],
        },
        system,
    )
    .is_err());
    assert_eq!(tx.len(), staged);
    assert!(associate_classification(
        &mut tx,
        &model,
        AssociationDraft {
            global_id: &gid(19),
            name: None,
            description: None,
            related_objects: &[wall, wall],
        },
        system,
    )
    .is_err());
    assert_eq!(tx.len(), staged);
    assert!(associate_classification(
        &mut tx,
        &model,
        AssociationDraft {
            global_id: &gid(20),
            name: None,
            description: None,
            related_objects: &[wall],
        },
        wall,
    )
    .is_err());
    assert_eq!(tx.len(), staged);
    let relation = associate_classification(
        &mut tx,
        &model,
        AssociationDraft {
            global_id: &gid(20),
            name: None,
            description: None,
            related_objects: &[wall],
        },
        system,
    )
    .unwrap();
    tx.set_attribute(relation, 5, Value::Ref(EntityId(9999)));
    assert!(tx.commit(&mut model).is_err());
    assert_eq!(ClassificationView::new(&model).systems().count(), 0);
}
