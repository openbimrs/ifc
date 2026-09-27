//! Fixtures for the predefined property set tests (#149): records built by
//! attribute name from each release's bundled table.
//!
//! Each test binary compiles this module separately, so a helper used by
//! only one of them is dead code in the other.
#![allow(dead_code)]

use ifc_model::{Codec, EntityId, Model};
use ifc_properties::{
    ExactProperty, ExactPropertyError, ExactResolution, ExactValue, SchemaVersion,
};
use ifc_schema::Schema;
use ifc_step::StepCodec;

pub const RELEASES: [(&str, SchemaVersion); 3] = [
    ("IFC2X3", SchemaVersion::Ifc2x3),
    ("IFC4", SchemaVersion::Ifc4),
    ("IFC4X3_ADD2", SchemaVersion::Ifc4x3),
];

pub const DOOR: EntityId = EntityId(1);
pub const DOOR_TYPE: EntityId = EntityId(2);

pub fn table(version: SchemaVersion) -> &'static Schema {
    ifc_schema::for_version(version).expect("bundled")
}

/// `#id=ENTITY(...)` with `values` in the named slots and `$` elsewhere.
pub fn record(schema: &Schema, id: u64, entity: &str, values: &[(&str, &str)]) -> String {
    let names = schema.attribute_names(entity);
    assert!(!names.is_empty(), "{entity} is declared");
    for (name, _) in values {
        assert!(names.contains(name), "{entity}.{name} is declared");
    }
    let slots: Vec<&str> = names
        .iter()
        .map(|name| {
            values
                .iter()
                .find(|(key, _)| key == name)
                .map_or("$", |(_, value)| value)
        })
        .collect();
    format!(
        "#{id}={}({});",
        entity.to_ascii_uppercase(),
        slots.join(",")
    )
}

pub fn guid(id: u64) -> String {
    format!("'{id:0>22}'")
}

/// A root entity (`GlobalId` stated) with further named values.
pub fn rooted(schema: &Schema, id: u64, entity: &str, values: &[(&str, &str)]) -> String {
    let global = guid(id);
    let mut all = vec![("GlobalId", global.as_str())];
    all.extend_from_slice(values);
    record(schema, id, entity, &all)
}

/// A door #1 typed by #2 (`IfcDoorStyle` in IFC2X3), a project #5 in
/// millimetres (#7), `occurrence` sets assigned to the door and `typed`
/// sets held by the type; `sets` are the set records themselves.
pub fn door_model(
    version: SchemaVersion,
    occurrence: &[u64],
    typed: &[u64],
    sets: &[String],
) -> Model {
    let schema = table(version);
    let refs = |ids: &[u64]| {
        let list: Vec<String> = ids.iter().map(|id| format!("#{id}")).collect();
        format!("({})", list.join(","))
    };
    let type_sets = if typed.is_empty() {
        "$".to_owned()
    } else {
        refs(typed)
    };
    let door_type = if version == SchemaVersion::Ifc2x3 {
        rooted(
            schema,
            2,
            "IfcDoorStyle",
            &[
                ("HasPropertySets", &type_sets),
                ("OperationType", ".DOUBLE_DOOR_SINGLE_SWING."),
                ("ConstructionType", ".WOOD."),
                ("ParameterTakesPrecedence", ".F."),
                ("Sizeable", ".F."),
            ],
        )
    } else {
        rooted(
            schema,
            2,
            "IfcDoorType",
            &[
                ("HasPropertySets", &type_sets),
                ("PredefinedType", ".DOOR."),
                ("OperationType", ".DOUBLE_DOOR_SINGLE_SWING."),
            ],
        )
    };
    let mut records = vec![
        rooted(schema, 1, "IfcDoor", &[("Name", "'Door'")]),
        door_type,
        rooted(
            schema,
            3,
            "IfcRelDefinesByType",
            &[("RelatedObjects", "(#1)"), ("RelatingType", "#2")],
        ),
        rooted(schema, 5, "IfcProject", &[("UnitsInContext", "#6")]),
        record(schema, 6, "IfcUnitAssignment", &[("Units", "(#7)")]),
        record(
            schema,
            7,
            "IfcSIUnit",
            &[
                ("Dimensions", "*"),
                ("UnitType", ".LENGTHUNIT."),
                ("Prefix", ".MILLI."),
                ("Name", ".METRE."),
            ],
        ),
    ];
    for (index, set) in occurrence.iter().enumerate() {
        let id = 30 + index as u64;
        let definition = format!("#{set}");
        records.push(rooted(
            schema,
            id,
            "IfcRelDefinesByProperties",
            &[
                ("RelatedObjects", "(#1)"),
                ("RelatingPropertyDefinition", &definition),
            ],
        ));
    }
    records.extend(sets.iter().cloned());
    parse(version, &records)
}

pub fn parse(version: SchemaVersion, records: &[String]) -> Model {
    let token = RELEASES
        .iter()
        .find(|(_, v)| *v == version)
        .map(|(t, _)| *t)
        .expect("release");
    let text = format!(
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
         FILE_NAME('','',(''),(''),'','','');\nFILE_SCHEMA(('{token}'));\nENDSEC;\n\
         DATA;\n{}\nENDSEC;\nEND-ISO-10303-21;\n",
        records.join("\n")
    );
    let model = StepCodec
        .read_bytes(text.as_bytes())
        .unwrap_or_else(|e| panic!("fixture must parse: {e:?}"));
    assert!(model.diagnostics().is_empty(), "{:?}", model.diagnostics());
    model
}

pub fn lining(version: SchemaVersion, id: u64, values: &[(&str, &str)]) -> String {
    rooted(table(version), id, "IfcDoorLiningProperties", values)
}

pub fn panel(version: SchemaVersion, id: u64, values: &[(&str, &str)]) -> String {
    rooted(table(version), id, "IfcDoorPanelProperties", values)
}

pub fn leaf(version: SchemaVersion, id: u64, position: &str) -> String {
    panel(
        version,
        id,
        &[
            ("PanelDepth", "40."),
            ("PanelOperation", ".SWINGING."),
            ("PanelWidth", "0.5"),
            ("PanelPosition", position),
        ],
    )
}

pub fn present(result: Result<ExactResolution, ExactPropertyError>) -> ExactProperty {
    match result {
        Ok(ExactResolution::Present(property)) => property,
        other => panic!("expected a present property, got {other:?}"),
    }
}

pub fn enum_value(text: &str) -> ExactValue {
    ExactValue::Enum(text.into())
}
