//! Header binding and record layout of the release a call binds to:
//! one release or a typed refusal, and records that refuse what the release
//! cannot hold.

use ifc_model::{Model, Value};
use ifc_schema::SchemaVersion;

use super::Release;
use crate::ClassificationError;

#[test]
fn the_header_binds_one_release_or_fails_closed() {
    let bind = |schemas: &[&str]| {
        let mut model = Model::new();
        model.header_mut().schema = schemas.iter().map(|s| (*s).to_owned()).collect();
        Release::of(&model).bound().map(|(version, _)| version)
    };
    assert_eq!(bind(&[]), Ok(SchemaVersion::Ifc4));
    assert_eq!(bind(&["IFC2X3"]), Ok(SchemaVersion::Ifc2x3));
    assert_eq!(bind(&["IFC4"]), Ok(SchemaVersion::Ifc4));
    assert_eq!(bind(&["IFC4X3"]), Ok(SchemaVersion::Ifc4x3));
    assert_eq!(bind(&["ifc4x3_add2"]), Ok(SchemaVersion::Ifc4x3));
    assert_eq!(
        bind(&["IFC4X1"]),
        Err(ClassificationError::UnsupportedSchema {
            schema: "IFC4X1".to_owned()
        })
    );
    assert_eq!(
        bind(&["IFC4", "IFC4X3"]),
        Err(ClassificationError::MultipleSchemas { schemas: 2 })
    );
}

#[test]
fn records_refuse_what_the_release_cannot_hold() {
    let ifc2x3 = Release::Bound(SchemaVersion::Ifc2x3);
    let text = |s: &str| Value::Text(s.into());
    assert_eq!(
        ifc2x3.record(
            "IFCCLASSIFICATION",
            vec![("Name", text("U")), ("Description", text("d"))]
        ),
        Err(ClassificationError::AuthoringNotInSchema {
            entity: "IFCCLASSIFICATION",
            attribute: "Description",
            schema: SchemaVersion::Ifc2x3,
        })
    );
    assert_eq!(
        ifc2x3.record(
            "IFCCLASSIFICATION",
            vec![
                ("Source", text("s")),
                ("Edition", text("e")),
                ("EditionDate", text("2024-01-01")),
                ("Name", text("U"))
            ]
        ),
        Err(ClassificationError::AuthoringValueType {
            entity: "IFCCLASSIFICATION",
            attribute: "EditionDate",
            declared: "IfcCalendarDate",
            schema: SchemaVersion::Ifc2x3,
        })
    );
    assert_eq!(
        ifc2x3.record("IFCCLASSIFICATION", vec![("Name", text("U"))]),
        Err(ClassificationError::AuthoringRequired {
            entity: "IFCCLASSIFICATION",
            attribute: "Source",
            schema: SchemaVersion::Ifc2x3,
        })
    );
    assert_eq!(
        ifc2x3.record("IFCEXTERNALREFERENCERELATIONSHIP", vec![]),
        Err(ClassificationError::EntityNotInSchema {
            entity: "IFCEXTERNALREFERENCERELATIONSHIP",
            schema: SchemaVersion::Ifc2x3,
        })
    );
    // IFC4X3 writes the IFC4-named Location into Specification.
    let record = Release::Bound(SchemaVersion::Ifc4x3)
        .record(
            "IFCCLASSIFICATION",
            vec![("Name", text("U")), ("Location", text("https://x"))],
        )
        .unwrap();
    assert_eq!(record.attributes.len(), 7);
    assert_eq!(record.attributes[5], text("https://x"));
}
