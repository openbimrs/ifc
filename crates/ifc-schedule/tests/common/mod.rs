//! Shared fixtures for the release-bound authoring tests (#202).
//!
//! Included with `mod common;`: integration tests are separate binaries, so
//! this is the standard way to share setup without a helper crate.

use ifc_model::{Codec, Edit, Entity, EntityId, Model, Transaction};
use ifc_schedule::SchemaVersion;
use ifc_schema::for_version;
use ifc_step::StepCodec;

pub const OWNER: EntityId = EntityId(5);
#[allow(dead_code)] // release_refusals.rs only
pub const WALL: EntityId = EntityId(10);
/// An IFC2X3 `IfcWorkSchedule`, present only in [`base`]'s IFC2X3 model:
/// this crate cannot author one there (its dates are `IfcDateTimeSelect`
/// records), but tasks can still be assigned to it.
#[allow(dead_code)] // release_authoring.rs only
pub const IFC2X3_SCHEDULE: EntityId = EntityId(21);

#[allow(dead_code)] // not every test binary uses every fixture
pub const RELEASES: [(&str, SchemaVersion); 3] = [
    ("IFC2X3", SchemaVersion::Ifc2x3),
    ("IFC4", SchemaVersion::Ifc4),
    ("IFC4X3_ADD2", SchemaVersion::Ifc4x3),
];

/// An owner history (`#5`) with its actors and a wall (`#10`); in IFC2X3
/// also a work schedule (`#21`) dated by an `IfcCalendarDate` (`#20`).
pub fn base(schema: &str, version: SchemaVersion) -> Model {
    let arity = for_version(version).unwrap().attributes("IFCWALL").len();
    let unset = ",$".repeat(arity - 3);
    let schedule = if version == SchemaVersion::Ifc2x3 {
        "#20=IFCCALENDARDATE(1,1,2026);\n\
         #21=IFCWORKSCHEDULE('2kTvXnbbzCWw8lcMd1dR4o',#5,'Programme',$,$,'WS-1',#20,$,$,$,$,#20,$,$,$);\n"
    } else {
        ""
    };
    let text = format!(
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
         FILE_NAME('','',(''),(''),'','','');\nFILE_SCHEMA(('{schema}'));\nENDSEC;\nDATA;\n\
         #1=IFCPERSON($,'Doe','Jane',$,$,$,$,$);\n\
         #2=IFCORGANIZATION($,'Acme',$,$,$);\n\
         #3=IFCPERSONANDORGANIZATION(#1,#2,$);\n\
         #4=IFCAPPLICATION(#2,'1.0','Test','test');\n\
         #5=IFCOWNERHISTORY(#3,#4,$,.NOCHANGE.,$,$,$,1700000000);\n\
         #10=IFCWALL('1xS3BCk291UvhgP2dvNsgp',#5,'W1'{unset});\n\
         {schedule}\
         ENDSEC;\nEND-ISO-10303-21;\n"
    );
    let model = StepCodec.read_bytes(text.as_bytes()).expect("parses");
    assert!(model.diagnostics().is_empty(), "{:?}", model.diagnostics());
    model
}

/// The record `tx` staged as `id`.
#[allow(dead_code)]
pub fn staged(tx: &Transaction, id: EntityId) -> Entity {
    tx.edits()
        .iter()
        .find_map(|edit| match edit {
            Edit::Create { id: staged, entity } if *staged == id => Some(entity.clone()),
            _ => None,
        })
        .expect("staged")
}
