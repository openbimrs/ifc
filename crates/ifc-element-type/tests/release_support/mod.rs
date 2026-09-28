//! Fixtures for the #202 release tests: a model declaring each release,
//! and the tables it is checked against.
//!
//! Each test binary compiles this module separately, so a helper used by
//! only one of them is dead code in the other.
#![allow(dead_code)]

use ifc_element_type::{ElementType, ElementTypeError, TypeDraft};
use ifc_model::{Codec, Edit, Entity, EntityId, Model, Transaction};
use ifc_schema::{for_version, Schema, SchemaVersion, TypeKind};
use ifc_step::StepCodec;

pub const RELEASES: [(&str, SchemaVersion); 3] = [
    ("IFC2X3", SchemaVersion::Ifc2x3),
    ("IFC4", SchemaVersion::Ifc4),
    ("IFC4X3_ADD2", SchemaVersion::Ifc4x3),
];
pub const OWNER: EntityId = EntityId(5);
pub const WALL: EntityId = EntityId(10);
pub const GUID: &str = "1hqA$FMcT8$hVvcqsRDBzZ";

/// Actors and an owner history (`#5`) and a wall (`#10`) declaring
/// `schema`; an empty `schema` leaves `FILE_SCHEMA` empty.
pub fn model(schema: &[&str]) -> Model {
    let version = match schema {
        [token] => SchemaVersion::from_header_token(token),
        _ => None,
    }
    .unwrap_or(SchemaVersion::Ifc4);
    let wall_tail = ",$".repeat(table(version).attributes("IFCWALL").len() - 3);
    let declared = schema
        .iter()
        .map(|token| format!("'{token}'"))
        .collect::<Vec<_>>()
        .join(",");
    let text = format!(
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
         FILE_NAME('','',(''),(''),'','','');\nFILE_SCHEMA(({declared}));\nENDSEC;\nDATA;\n\
         #1=IFCPERSON($,'Doe','Jane',$,$,$,$,$);\n\
         #2=IFCORGANIZATION($,'Acme',$,$,$);\n\
         #3=IFCPERSONANDORGANIZATION(#1,#2,$);\n\
         #4=IFCAPPLICATION(#2,'1.0','Test','test');\n\
         #5=IFCOWNERHISTORY(#3,#4,$,.NOCHANGE.,$,$,$,1700000000);\n\
         #10=IFCWALL('1xS3BCk291UvhgP2dvNsgp',#5,'W1'{wall_tail});\n\
         ENDSEC;\nEND-ISO-10303-21;\n"
    );
    StepCodec.read_bytes(text.as_bytes()).expect("parses")
}

pub fn table(version: SchemaVersion) -> &'static Schema {
    for_version(version).expect("bundled")
}

pub fn declares(schema: &Schema, entity: &str) -> bool {
    schema.entity(entity).is_some_and(|found| !found.abstract_)
}

pub fn slot(schema: &Schema, entity: &str, attribute: &str) -> usize {
    schema
        .attribute_names(entity)
        .iter()
        .position(|name| *name == attribute)
        .unwrap_or_else(|| panic!("{entity}.{attribute}"))
}

/// `entity`'s `PredefinedType` tokens in `schema`, if it declares one.
pub fn members(schema: &Schema, entity: &str) -> Option<Vec<String>> {
    let declared = schema
        .attributes(entity)
        .into_iter()
        .find(|attribute| attribute.name == "PredefinedType")?;
    match &schema.type_def(&declared.type_name)?.kind {
        TypeKind::Enumeration(members) => Some(members.clone()),
        _ => None,
    }
}

/// The first token this release declares for `kind`, other than
/// `USERDEFINED`, so the draft needs no fallback.
pub fn token(schema: &Schema, kind: ElementType) -> Option<String> {
    members(schema, kind.type_name)?
        .into_iter()
        .find(|member| member != "USERDEFINED")
}

pub fn named() -> TypeDraft<'static> {
    TypeDraft {
        name: Some("Sweep"),
        tag_or_long_description: Some("T-1"),
        ..TypeDraft::default()
    }
}

/// The one record `tx` staged.
pub fn staged(tx: &Transaction) -> &Entity {
    match tx.edits() {
        [Edit::Create { entity, .. }] => entity,
        other => panic!("expected one create, got {other:?}"),
    }
}

/// Refused before anything is staged.
pub fn refused(
    model: &Model,
    author: impl Fn(&mut Transaction) -> Result<EntityId, ElementTypeError>,
) -> ElementTypeError {
    let mut tx = Transaction::new(model);
    let error = author(&mut tx).expect_err("refused");
    assert!(tx.is_empty(), "a refusal staged {:?}", tx.edits());
    error
}
