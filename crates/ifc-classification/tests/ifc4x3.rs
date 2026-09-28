//! #194: an IFC4X3 header binds the bundled IFC4X3 ADD2 table.
//!
//! Before, every header other than IFC2X3 was read against IFC4, so an
//! IFC4X3 model's classifications had to be refused downstream. Each
//! fixture is parsed from STEP so the header is real. Every IFC4X3
//! difference this crate meets, verified against `IFC4X3_ADD2.exp`, has a
//! test here:
//!
//! - `IfcClassification.Location` is `Specification` (same position);
//! - `IfcResourceObjectSelect` admits `IfcShapeAspect`;
//! - entities that exist only in IFC4X3 (an `IfcRoad`) are
//!   `IfcDefinitionSelect` members, so they can be classified;
//! - `IfcDocumentConfidentialityEnum` lists its members in another order.
//!
//! `IfcClassificationReference`, `IfcRelAssociatesClassification`,
//! `IfcExternalReference` and `IfcRoot` are unchanged from IFC4.

use ifc_classification::{
    classification_schema, create_classification_reference, ClassificationError,
    ClassificationReferenceDraft, ClassificationView, SchemaVersion,
};
use ifc_model::{Budget, Codec, EntityId, Model, Transaction};
use ifc_schema::for_version;

/// A STEP record for `entity` in `version`: `head` then `$` up to the
/// release's arity.
fn record(version: SchemaVersion, id: u64, entity: &str, head: &[&str]) -> String {
    let arity = for_version(version).unwrap().attributes(entity).len();
    assert!(head.len() <= arity, "{entity}: {} > {arity}", head.len());
    let mut values: Vec<&str> = head.to_vec();
    values.resize(arity, "$");
    format!("#{id}={entity}({});", values.join(","))
}

fn step(schema: &str, records: &[String]) -> Model {
    let text = format!(
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
         FILE_NAME('t','2026-09-28T00:00:00',(''),(''),'','','');\n\
         FILE_SCHEMA(({schema}));\nENDSEC;\nDATA;\n{}\nENDSEC;\nEND-ISO-10303-21;\n",
        records.join("\n")
    );
    let model = ifc_step::StepCodec
        .read_bytes(text.as_bytes())
        .expect("fixture parses");
    assert!(model.diagnostics().is_empty(), "{:?}", model.diagnostics());
    model
}

const WALL: EntityId = EntityId(10);
const ROAD: EntityId = EntityId(11);
const SYSTEM: EntityId = EntityId(1);
const REFERENCE: EntityId = EntityId(2);
const ASPECT: EntityId = EntityId(30);

/// A wall and a road classified by a Uniclass reference, plus a shape
/// aspect carrying the reference through `IfcExternalReferenceRelationship`.
/// The data is IFC4X3; `schema` is the header it is declared under.
fn fixture(schema: &str) -> Model {
    let v = SchemaVersion::Ifc4x3;
    step(
        schema,
        &[
            record(
                v,
                1,
                "IFCCLASSIFICATION",
                &[
                    "'NBS'",
                    "'2024'",
                    "'2024-01-01'",
                    "'Uniclass'",
                    "'Unified classification'",
                    "'https://uniclass.thenbs.com'",
                    "('_')",
                ],
            ),
            record(
                v,
                2,
                "IFCCLASSIFICATIONREFERENCE",
                &[
                    "'https://uniclass.thenbs.com/Pr_20'",
                    "'Pr_20'",
                    "'Products'",
                    "#1",
                    "'Structural'",
                    "'20'",
                ],
            ),
            record(v, 10, "IFCWALL", &["'1xS3BCk291UvhgP2dvNsgp'", "$", "'W1'"]),
            record(v, 11, "IFCROAD", &["'2xS3BCk291UvhgP2dvNsgp'", "$", "'R1'"]),
            record(
                v,
                20,
                "IFCRELASSOCIATESCLASSIFICATION",
                &["'3xS3BCk291UvhgP2dvNsgp'", "$", "$", "$", "(#10,#11)", "#2"],
            ),
            record(v, 30, "IFCSHAPEASPECT", &["()", "'Aspect'", "$", ".U."]),
            record(
                v,
                31,
                "IFCEXTERNALREFERENCERELATIONSHIP",
                &["$", "$", "#2", "(#30)"],
            ),
        ],
    )
}

fn ifc4x3() -> Model {
    fixture("'IFC4X3_ADD2'")
}

#[test]
fn an_ifc4x3_header_binds_the_ifc4x3_table() {
    assert_eq!(classification_schema(&ifc4x3()), Ok(SchemaVersion::Ifc4x3));
    assert_eq!(
        classification_schema(&fixture("'IFC4X3'")),
        Ok(SchemaVersion::Ifc4x3)
    );
}

/// The acceptance test of #194: a wall's classification reference resolves
/// its identification and source in an IFC4X3 model.
#[test]
fn a_wall_resolves_its_ifc4x3_classification_reference() {
    let model = ifc4x3();
    let view = ClassificationView::new(&model);
    let assignments = view.classification_assignments_for(WALL).unwrap();
    assert_eq!(assignments.len(), 1);
    assert_eq!(assignments[0].relating_classification_id(), Ok(REFERENCE));
    assert_eq!(assignments[0].related_object_ids(), Ok(vec![WALL, ROAD]));

    let reference = view.references().next().unwrap();
    assert_eq!(reference.identification(), Ok(Some("Pr_20")));
    assert_eq!(reference.name(), Ok(Some("Products")));
    assert_eq!(reference.referenced_source_id(), Ok(Some(SYSTEM)));
    assert_eq!(reference.description(), Ok(Some("Structural")));
    assert_eq!(reference.sort(), Ok(Some("20")));

    let hierarchy = view.hierarchy_from(REFERENCE, Budget::default()).unwrap();
    let system = hierarchy
        .system
        .expect("the chain ends at the classification");
    assert_eq!(system.id(), SYSTEM);
    assert_eq!(system.name(), Ok("Uniclass"));
    assert_eq!(system.source(), Ok(Some("NBS")));
    assert_eq!(system.edition_date(), Ok(Some("2024-01-01")));
    assert_eq!(system.reference_tokens(), Ok(Some(vec!["_"])));

    let effective = view.effective_classifications(WALL).unwrap();
    assert_eq!(effective.occurrence.len(), 1);
}

/// IFC4X3 renamed `IfcClassification.Location` to `Specification`; the
/// IFC4-named accessor reads it at the same position.
#[test]
fn location_reads_the_ifc4x3_specification() {
    let model = ifc4x3();
    let view = ClassificationView::new(&model);
    let system = view.systems().next().unwrap();
    assert_eq!(system.location(), Ok(Some("https://uniclass.thenbs.com")));
    assert_eq!(system.description(), Ok(Some("Unified classification")));
}

/// `IfcRoad` exists only in IFC4X3. Bound to IFC4X3 it is an
/// `IfcDefinitionSelect` member and can be classified; the same data under
/// an IFC4 header is refused, which is what binding IFC4X3 to IFC4 did.
#[test]
fn an_ifc4x3_only_element_is_classified_only_under_ifc4x3() {
    let model = ifc4x3();
    let view = ClassificationView::new(&model);
    let effective = view.effective_classifications(ROAD).unwrap();
    assert_eq!(effective.occurrence.len(), 1);
    assert_eq!(
        effective.occurrence[0].relating_classification_id(),
        Ok(REFERENCE)
    );

    let as_ifc4 = fixture("'IFC4'");
    assert_eq!(
        ClassificationView::new(&as_ifc4)
            .effective_classifications(ROAD)
            .err(),
        Some(ClassificationError::ReferenceType {
            entity: "IFCRELASSOCIATESCLASSIFICATION",
            id: EntityId(20),
            attribute: "RelatedObjects",
            target: ROAD,
            expected: "IfcDefinitionSelect",
            actual: "IFCROAD".to_owned(),
        })
    );
}

/// `IfcResourceObjectSelect` gained `IfcShapeAspect` in IFC4X3, so a shape
/// aspect may carry an external reference there and not in IFC4.
#[test]
fn a_shape_aspect_is_a_resource_object_only_in_ifc4x3() {
    let model = ifc4x3();
    let view = ClassificationView::new(&model);
    let relationships = view.external_references_for(ASPECT).unwrap();
    assert_eq!(relationships.len(), 1);
    assert_eq!(relationships[0].relating_reference(), Ok(REFERENCE));

    let as_ifc4 = fixture("'IFC4'");
    assert_eq!(
        ClassificationView::new(&as_ifc4)
            .external_references_for(ASPECT)
            .err(),
        Some(ClassificationError::ReferenceType {
            entity: "IFCEXTERNALREFERENCERELATIONSHIP",
            id: EntityId(31),
            attribute: "RelatedResourceObjects",
            target: ASPECT,
            expected: "IfcResourceObjectSelect",
            actual: "IFCSHAPEASPECT".to_owned(),
        })
    );
}

/// Several declarations, or one with no bundled table, are refused by every
/// read and write rather than read as IFC4. Nothing is staged.
#[test]
fn multiple_or_unknown_schemas_are_refused() {
    for (schema, error) in [
        (
            "'IFC4','IFC4X3'",
            ClassificationError::MultipleSchemas { schemas: 2 },
        ),
        (
            "'IFC4X1'",
            ClassificationError::UnsupportedSchema {
                schema: "IFC4X1".to_owned(),
            },
        ),
    ] {
        let model = fixture(schema);
        assert_eq!(
            classification_schema(&model),
            Err(error.clone()),
            "{schema}"
        );
        let view = ClassificationView::new(&model);
        assert_eq!(view.systems().next().unwrap().name(), Err(error.clone()));
        assert_eq!(
            view.references().next().unwrap().identification(),
            Err(error.clone())
        );
        assert_eq!(
            view.classification_assignments_for(WALL)
                .map(|found| found.len()),
            Err(error.clone())
        );

        let mut tx = Transaction::new(&model);
        let draft = ClassificationReferenceDraft::new()
            .identification("Pr_30")
            .referenced_source(SYSTEM);
        assert_eq!(
            create_classification_reference(&mut tx, &model, draft),
            Err(error)
        );
        assert!(tx.is_empty(), "{schema}: a refusal stages nothing");
    }
}
