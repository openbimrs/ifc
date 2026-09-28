//! Shared fixtures for the authoring test binaries.
//!
//! Included with `mod common;`: integration tests are separate binaries, so
//! this is the standard way to share setup without a helper crate.

use ifc_classification::{AssociationDraft, SchemaVersion};
use ifc_model::{Codec, EntityId, Model};
use ifc_schema::for_version;

pub const PERSON: EntityId = EntityId(1);
pub const ORGANIZATION: EntityId = EntityId(2);
pub const OWNER: EntityId = EntityId(5);
pub const WALL: EntityId = EntityId(10);

#[allow(dead_code)] // used by release_authoring.rs only
pub const RELEASES: [(&str, SchemaVersion); 3] = [
    ("IFC2X3", SchemaVersion::Ifc2x3),
    ("IFC4", SchemaVersion::Ifc4),
    ("IFC4X3_ADD2", SchemaVersion::Ifc4x3),
];

/// An owner history (`#5`) with its actors and a wall (`#10`).
pub fn base(schema: &str, version: SchemaVersion) -> Model {
    let arity = |entity: &str| for_version(version).unwrap().attributes(entity).len();
    let unset = ",$".repeat(arity("IFCWALL") - 3);
    let text = format!(
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
         FILE_NAME('','',(''),(''),'','','');\nFILE_SCHEMA(('{schema}'));\nENDSEC;\nDATA;\n\
         #1=IFCPERSON($,'Doe','Jane',$,$,$,$,$);\n\
         #2=IFCORGANIZATION($,'Acme',$,$,$);\n\
         #3=IFCPERSONANDORGANIZATION(#1,#2,$);\n\
         #4=IFCAPPLICATION(#2,'1.0','Test','test');\n\
         #5=IFCOWNERHISTORY(#3,#4,$,.NOCHANGE.,$,$,$,1700000000);\n\
         #10=IFCWALL('1xS3BCk291UvhgP2dvNsgp',#5,'W1'{unset});\n\
         ENDSEC;\nEND-ISO-10303-21;\n"
    );
    let model = ifc_step::StepCodec
        .read_bytes(text.as_bytes())
        .expect("parses");
    assert!(model.diagnostics().is_empty(), "{:?}", model.diagnostics());
    model
}

pub fn relation(global_id: &str) -> AssociationDraft<'_> {
    AssociationDraft {
        global_id,
        name: None,
        description: None,
        related_objects: &[WALL],
    }
}
