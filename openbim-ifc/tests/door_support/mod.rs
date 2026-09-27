//! Door fixtures for the operation-geometry tests (#148): STEP records built
//! by attribute name from each release's bundled table, so no slot number is
//! restated here.
//!
//! Each test binary compiles this module separately, so a helper used by
//! only one of them is dead code in the other.
#![allow(dead_code)]

use ifc::{Codec, Model, StepCodec};
use ifc_properties::SchemaVersion;
use ifc_schema::Schema;

pub use ifc_model::EntityId;

pub const DOOR: EntityId = EntityId(1);
pub const DOOR_TYPE: EntityId = EntityId(2);

pub fn table(version: SchemaVersion) -> &'static Schema {
    ifc_schema::for_version(version).expect("bundled")
}

fn token(version: SchemaVersion) -> &'static str {
    match version {
        SchemaVersion::Ifc2x3 => "IFC2X3",
        SchemaVersion::Ifc4 => "IFC4",
        _ => "IFC4X3_ADD2",
    }
}

/// `#id=ENTITY(...)` with `values` in the named slots and `$` elsewhere.
pub fn record(version: SchemaVersion, id: u64, entity: &str, values: &[(&str, &str)]) -> String {
    let names = table(version).attribute_names(entity);
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

/// A root entity (`GlobalId` stated) with further named values.
pub fn rooted(version: SchemaVersion, id: u64, entity: &str, values: &[(&str, &str)]) -> String {
    let global = format!("'{id:0>22}'");
    let mut all = vec![("GlobalId", global.as_str())];
    all.extend_from_slice(values);
    record(version, id, entity, &all)
}

/// One `IfcDoorPanelProperties` record.
pub fn panel(
    version: SchemaVersion,
    id: u64,
    operation: &str,
    position: &str,
    width: Option<&str>,
) -> String {
    let operation = format!(".{operation}.");
    let position = format!(".{position}.");
    let mut values = vec![
        ("PanelOperation", operation.as_str()),
        ("PanelPosition", position.as_str()),
    ];
    if let Some(width) = width {
        values.push(("PanelWidth", width));
    }
    rooted(version, id, "IfcDoorPanelProperties", &values)
}

/// A door #1 placed by #12 in a project #5 whose length unit is #7.
#[derive(Clone)]
pub struct Door {
    pub version: SchemaVersion,
    /// `IfcSIPrefix` of the length unit, e.g. `MILLI`; `None` for metres.
    pub prefix: Option<&'static str>,
    /// `OverallWidth` as written, e.g. `0.9`; `None` for `$`.
    pub width: Option<&'static str>,
    /// `IfcDoor.OperationType` (IFC4, IFC4X3).
    pub occurrence_operation: Option<&'static str>,
    /// The type object's `OperationType`; `None` for an untyped door.
    pub type_operation: Option<&'static str>,
    /// Entity name of the type object, when it is not the release's own.
    pub type_entity: Option<&'static str>,
    /// Entity name of the occurrence, when it is not `IfcDoor`.
    pub door_entity: &'static str,
    /// Panel records held by the type object's `HasPropertySets`.
    pub type_panels: Vec<(u64, String)>,
    /// Panel records assigned to the occurrence.
    pub occurrence_panels: Vec<(u64, String)>,
    /// Placement `Location`, `Axis` and `RefDirection`.
    pub location: &'static str,
    pub axis: &'static str,
    pub ref_direction: &'static str,
}

impl Door {
    /// An IFC4 door in metres at the origin, typed `operation`, with the
    /// given type panels and width 0.9.
    pub fn new(
        version: SchemaVersion,
        operation: &'static str,
        panels: Vec<(u64, String)>,
    ) -> Self {
        Self {
            version,
            prefix: None,
            width: Some("0.9"),
            occurrence_operation: None,
            type_operation: Some(operation),
            type_entity: None,
            door_entity: "IfcDoor",
            type_panels: panels,
            occurrence_panels: Vec::new(),
            location: "(0.,0.,0.)",
            axis: "(0.,0.,1.)",
            ref_direction: "(1.,0.,0.)",
        }
    }

    pub fn model(&self) -> Model {
        let v = self.version;
        let refs = |ids: &[u64]| {
            let list: Vec<String> = ids.iter().map(|id| format!("#{id}")).collect();
            format!("({})", list.join(","))
        };
        let width = self.width.unwrap_or("$");
        let mut door_values = vec![
            ("Name", "'Door'"),
            ("ObjectPlacement", "#12"),
            ("OverallWidth", width),
        ];
        let occurrence_operation = self.occurrence_operation.map(|op| format!(".{op}."));
        if let Some(op) = &occurrence_operation {
            door_values.push(("OperationType", op.as_str()));
        }
        let prefix = self.prefix.map(|p| format!(".{p}."));
        let mut records = vec![
            rooted(v, 1, self.door_entity, &door_values),
            rooted(v, 5, "IfcProject", &[("UnitsInContext", "#6")]),
            record(v, 6, "IfcUnitAssignment", &[("Units", "(#7)")]),
            record(
                v,
                7,
                "IfcSIUnit",
                &[
                    ("Dimensions", "*"),
                    ("UnitType", ".LENGTHUNIT."),
                    ("Prefix", prefix.as_deref().unwrap_or("$")),
                    ("Name", ".METRE."),
                ],
            ),
            record(v, 8, "IfcCartesianPoint", &[("Coordinates", self.location)]),
            record(v, 9, "IfcDirection", &[("DirectionRatios", self.axis)]),
            record(
                v,
                10,
                "IfcDirection",
                &[("DirectionRatios", self.ref_direction)],
            ),
            record(
                v,
                11,
                "IfcAxis2Placement3D",
                &[("Location", "#8"), ("Axis", "#9"), ("RefDirection", "#10")],
            ),
            record(v, 12, "IfcLocalPlacement", &[("RelativePlacement", "#11")]),
        ];
        if let Some(operation) = self.type_operation {
            let ids: Vec<u64> = self.type_panels.iter().map(|(id, _)| *id).collect();
            let sets = if ids.is_empty() {
                "$".to_owned()
            } else {
                refs(&ids)
            };
            let operation = format!(".{operation}.");
            let style = self.type_entity.unwrap_or(match v {
                SchemaVersion::Ifc2x3 => "IfcDoorStyle",
                _ => "IfcDoorType",
            });
            let mut values = vec![
                ("HasPropertySets", sets.as_str()),
                ("OperationType", operation.as_str()),
            ];
            if style == "IfcDoorStyle" {
                values.extend([
                    ("ConstructionType", ".WOOD."),
                    ("ParameterTakesPrecedence", ".F."),
                    ("Sizeable", ".F."),
                ]);
            } else {
                values.push(("PredefinedType", ".DOOR."));
            }
            records.push(rooted(v, 2, style, &values));
            records.push(rooted(
                v,
                3,
                "IfcRelDefinesByType",
                &[("RelatedObjects", "(#1)"), ("RelatingType", "#2")],
            ));
        }
        for (index, (id, _)) in self.occurrence_panels.iter().enumerate() {
            let definition = format!("#{id}");
            records.push(rooted(
                v,
                30 + index as u64,
                "IfcRelDefinesByProperties",
                &[
                    ("RelatedObjects", "(#1)"),
                    ("RelatingPropertyDefinition", &definition),
                ],
            ));
        }
        records.extend(self.type_panels.iter().map(|(_, record)| record.clone()));
        records.extend(
            self.occurrence_panels
                .iter()
                .map(|(_, record)| record.clone()),
        );
        parse(v, &records)
    }
}

pub fn parse(version: SchemaVersion, records: &[String]) -> Model {
    let text = format!(
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
         FILE_NAME('','',(''),(''),'','','');\nFILE_SCHEMA(('{}'));\nENDSEC;\n\
         DATA;\n{}\nENDSEC;\nEND-ISO-10303-21;\n",
        token(version),
        records.join("\n")
    );
    let model = StepCodec
        .read_bytes(text.as_bytes())
        .unwrap_or_else(|e| panic!("fixture must parse: {e:?}"));
    assert!(model.diagnostics().is_empty(), "{:?}", model.diagnostics());
    model
}

/// Component-wise closeness at 1e-12.
pub fn close(a: [f64; 3], b: [f64; 3]) -> bool {
    a.iter().zip(b).all(|(p, q)| (p - q).abs() < 1e-12)
}

#[track_caller]
pub fn assert_close(a: [f64; 3], b: [f64; 3]) {
    assert!(close(a, b), "{a:?} != {b:?}");
}

#[track_caller]
pub fn assert_near(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-12, "{a} != {b}");
}
