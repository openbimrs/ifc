//! Window fixtures for the operation-geometry tests (#170): STEP records
//! built by attribute name from each release's bundled table, with the
//! record helpers of `door_support`, so no slot number is restated here.
//!
//! Each test binary compiles this module separately, so a helper used by
//! only one of them is dead code in the other.
#![allow(dead_code)]

use ifc::Model;
use ifc_properties::SchemaVersion;

use crate::door_support::{parse, record, rooted};

pub use crate::door_support::EntityId;

pub const WINDOW: EntityId = EntityId(1);
pub const WINDOW_TYPE: EntityId = EntityId(2);

/// One `IfcWindowPanelProperties` record.
pub fn panel(version: SchemaVersion, id: u64, operation: &str, position: &str) -> String {
    framed_panel(version, id, operation, position, None)
}

/// One `IfcWindowPanelProperties` record with `(FrameDepth, FrameThickness)`.
pub fn framed_panel(
    version: SchemaVersion,
    id: u64,
    operation: &str,
    position: &str,
    frame: Option<(&str, &str)>,
) -> String {
    let operation = format!(".{operation}.");
    let position = format!(".{position}.");
    let mut values = vec![
        ("OperationType", operation.as_str()),
        ("PanelPosition", position.as_str()),
    ];
    if let Some((depth, thickness)) = frame {
        values.extend([("FrameDepth", depth), ("FrameThickness", thickness)]);
    }
    rooted(version, id, "IfcWindowPanelProperties", &values)
}

/// One `IfcWindowLiningProperties` record with the named offsets.
pub fn lining(version: SchemaVersion, id: u64, offsets: &[(&str, &str)]) -> String {
    rooted(version, id, "IfcWindowLiningProperties", offsets)
}

/// A window #1 placed by #12 in a project #5 whose length unit is #7.
#[derive(Clone)]
pub struct Window {
    pub version: SchemaVersion,
    /// `IfcSIPrefix` of the length unit, e.g. `MILLI`; `None` for metres.
    pub prefix: Option<&'static str>,
    /// `OverallWidth` as written; `None` for `$`.
    pub width: Option<&'static str>,
    /// `OverallHeight` as written; `None` for `$`.
    pub height: Option<&'static str>,
    /// `IfcWindow.PartitioningType` (IFC4, IFC4X3).
    pub occurrence_partitioning: Option<&'static str>,
    /// The type object's partitioning; `None` for an untyped window.
    pub type_partitioning: Option<&'static str>,
    /// Entity name of the type object, when it is not the release's own.
    pub type_entity: Option<&'static str>,
    /// Entity name of the occurrence, when it is not `IfcWindow`.
    pub window_entity: &'static str,
    /// Panel and lining records held by the type object's `HasPropertySets`.
    pub type_sets: Vec<(u64, String)>,
    /// Panel and lining records assigned to the occurrence.
    pub occurrence_sets: Vec<(u64, String)>,
    /// Placement `Location`, `Axis` and `RefDirection`.
    pub location: &'static str,
    pub axis: &'static str,
    pub ref_direction: &'static str,
}

impl Window {
    /// A window in metres at the origin, 1.2 wide and 1.5 high, typed
    /// `partitioning`, with the given type sets.
    pub fn new(
        version: SchemaVersion,
        partitioning: &'static str,
        sets: Vec<(u64, String)>,
    ) -> Self {
        Self {
            version,
            prefix: None,
            width: Some("1.2"),
            height: Some("1.5"),
            occurrence_partitioning: None,
            type_partitioning: Some(partitioning),
            type_entity: None,
            window_entity: "IfcWindow",
            type_sets: sets,
            occurrence_sets: Vec::new(),
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
        let mut window_values = vec![
            ("Name", "'Window'"),
            ("ObjectPlacement", "#12"),
            ("OverallWidth", self.width.unwrap_or("$")),
            ("OverallHeight", self.height.unwrap_or("$")),
        ];
        let occurrence = self.occurrence_partitioning.map(|p| format!(".{p}."));
        if let Some(partitioning) = &occurrence {
            window_values.push(("PartitioningType", partitioning.as_str()));
        }
        let prefix = self.prefix.map(|p| format!(".{p}."));
        let mut records = vec![
            rooted(v, 1, self.window_entity, &window_values),
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
        if let Some(partitioning) = self.type_partitioning {
            let ids: Vec<u64> = self.type_sets.iter().map(|(id, _)| *id).collect();
            let sets = if ids.is_empty() {
                "$".to_owned()
            } else {
                refs(&ids)
            };
            let partitioning = format!(".{partitioning}.");
            let style = self.type_entity.unwrap_or(match v {
                SchemaVersion::Ifc2x3 => "IfcWindowStyle",
                _ => "IfcWindowType",
            });
            let mut values = vec![("HasPropertySets", sets.as_str())];
            if style == "IfcWindowStyle" {
                values.extend([
                    ("ConstructionType", ".WOOD."),
                    ("OperationType", partitioning.as_str()),
                    ("ParameterTakesPrecedence", ".F."),
                    ("Sizeable", ".F."),
                ]);
            } else {
                values.extend([
                    ("PredefinedType", ".WINDOW."),
                    ("PartitioningType", partitioning.as_str()),
                ]);
            }
            records.push(rooted(v, 2, style, &values));
            records.push(rooted(
                v,
                3,
                "IfcRelDefinesByType",
                &[("RelatedObjects", "(#1)"), ("RelatingType", "#2")],
            ));
        }
        for (index, (id, _)) in self.occurrence_sets.iter().enumerate() {
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
        records.extend(self.type_sets.iter().map(|(_, record)| record.clone()));
        records.extend(
            self.occurrence_sets
                .iter()
                .map(|(_, record)| record.clone()),
        );
        parse(v, &records)
    }
}
