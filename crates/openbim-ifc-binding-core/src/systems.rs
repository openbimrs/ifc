//! Distribution systems and their membership (feature `systems`, #123).
//!
//! The facade's `systems` reader finds every `IfcSystem` subtype the
//! declared release has (an `IfcZone` is one in IFC4, not in IFC2X3), with
//! its members and the structures it serves. Memberships the file states
//! but cannot support are reported as anomalies with a stable `kind`, never
//! dropped silently. Records are read against the release the header
//! declares, IFC2X3, IFC4 or IFC4X3; any other is refused with
//! `unsupported-schema`.

use crate::record::{Field, Record, ToRecord};
use crate::{BindingError, IfcModel};

/// Every system in a model, and what the reader could not place.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Systems {
    /// The systems, by entity id.
    pub systems: Vec<System>,
    /// Memberships the file states but the schema forbids or the file
    /// cannot resolve.
    pub anomalies: Vec<SystemAnomaly>,
}

/// One system.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct System {
    /// Its entity id.
    pub id: u64,
    /// Its `GlobalId`.
    pub global_id: Option<String>,
    /// Its entity type, upper-case: `IFCDISTRIBUTIONSYSTEM`, ...
    pub type_name: String,
    /// `Name`.
    pub name: Option<String>,
    /// `IfcDistributionSystem.LongName` (IFC4, IFC4X3).
    pub long_name: Option<String>,
    /// `IfcDistributionSystem.PredefinedType`, e.g. `HEATING`.
    pub predefined_type: Option<String>,
    /// Members (`IfcRelAssignsToGroup`), in file order.
    pub members: Vec<u64>,
    /// Spatial structures it serves (`IfcRelServicesBuildings`).
    pub serviced_buildings: Vec<u64>,
    /// IFC4X3: spatial elements referencing it (`ServicesFacilities`).
    pub serviced_facilities: Vec<u64>,
}

/// A membership the reader could not honour.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemAnomaly {
    /// `dangling`, `zone-member-not-spatial`, `contained-twice`,
    /// `port-attached-twice`, `not-a-port`, `not-a-system`,
    /// `services-buildings-twice` or `serviced-not-spatial`.
    pub kind: String,
    /// The relationship (or, for the `-twice` kinds, the entity) at fault.
    pub subject: u64,
    /// The other entity involved: the missing id, the rejected member,
    /// structure, element or relationship.
    pub other: Option<u64>,
    /// A one-line explanation.
    pub message: String,
}

impl IfcModel {
    /// Every system, with its members and served structures.
    ///
    /// Refused with `unsupported-schema` for a release other than IFC2X3,
    /// IFC4 or IFC4X3, and with `feature-disabled` without the `systems`
    /// feature.
    pub fn systems(&self) -> Result<Systems, BindingError> {
        #[cfg(feature = "systems")]
        {
            read(self)
        }
        #[cfg(not(feature = "systems"))]
        {
            Err(BindingError::FeatureDisabled("systems"))
        }
    }
}

#[cfg(feature = "systems")]
fn read(model: &IfcModel) -> Result<Systems, BindingError> {
    use ifc::systems::{SchemaResolutionError, SystemAnomaly as Anomaly};

    let (systems, anomalies) =
        ifc::systems::systems(&model.inner).map_err(|error| match error {
            SchemaResolutionError::UnsupportedSchema { schema } => {
                BindingError::UnsupportedSchema(schema)
            }
            SchemaResolutionError::MissingSchema => BindingError::UnsupportedSchema(String::new()),
            other => BindingError::UnsupportedSchema(other.to_string()),
        })?;
    let mut out = Vec::with_capacity(systems.len());
    for system in systems {
        let identity = model.identity(system.id.0)?;
        out.push(System {
            id: system.id.0,
            global_id: identity.global_id,
            type_name: system.type_name,
            name: system.name,
            long_name: system.long_name,
            predefined_type: system.predefined_type,
            members: system.members.iter().map(|id| id.0).collect(),
            serviced_buildings: system.serviced_buildings.iter().map(|id| id.0).collect(),
            serviced_facilities: system.serviced_facilities.iter().map(|id| id.0).collect(),
        });
    }
    let anomalies = anomalies
        .into_iter()
        .map(|anomaly| {
            let message = format!("{anomaly:?}");
            let (kind, subject, other) = match anomaly {
                Anomaly::Dangling { relation, missing } => ("dangling", relation, Some(missing)),
                Anomaly::ZoneMemberNotSpatial {
                    relation, member, ..
                } => ("zone-member-not-spatial", relation, Some(member)),
                Anomaly::ContainedTwice {
                    element, second, ..
                } => ("contained-twice", element, Some(second)),
                Anomaly::PortAttachedTwice { port, rejected, .. } => {
                    ("port-attached-twice", port, Some(rejected))
                }
                Anomaly::NotAPort {
                    relation, entity, ..
                } => ("not-a-port", relation, Some(entity)),
                Anomaly::NotASystem {
                    relation, group, ..
                } => ("not-a-system", relation, Some(group)),
                Anomaly::ServicesBuildingsTwice {
                    system, rejected, ..
                } => ("services-buildings-twice", system, Some(rejected)),
                Anomaly::ServicedNotSpatial {
                    relation, target, ..
                } => ("serviced-not-spatial", relation, Some(target)),
                // An anomaly added after this binding: the message names it.
                _ => ("other", ifc::EntityId(0), None),
            };
            SystemAnomaly {
                kind: kind.to_owned(),
                subject: subject.0,
                other: other.map(|id| id.0),
                message,
            }
        })
        .collect();
    Ok(Systems {
        systems: out,
        anomalies,
    })
}

impl ToRecord for Systems {
    fn to_record(&self) -> Record {
        Record::new(
            "Systems",
            vec![
                ("systems", Field::records(&self.systems)),
                ("anomalies", Field::records(&self.anomalies)),
            ],
        )
    }
}

impl ToRecord for System {
    fn to_record(&self) -> Record {
        Record::new(
            "System",
            vec![
                ("id", Field::Id(self.id)),
                ("global_id", Field::text(self.global_id.clone())),
                ("type_name", Field::Text(self.type_name.clone())),
                ("name", Field::text(self.name.clone())),
                ("long_name", Field::text(self.long_name.clone())),
                ("predefined_type", Field::text(self.predefined_type.clone())),
                ("members", Field::ids(self.members.iter().copied())),
                (
                    "serviced_buildings",
                    Field::ids(self.serviced_buildings.iter().copied()),
                ),
                (
                    "serviced_facilities",
                    Field::ids(self.serviced_facilities.iter().copied()),
                ),
            ],
        )
    }
}

impl ToRecord for SystemAnomaly {
    fn to_record(&self) -> Record {
        Record::new(
            "SystemAnomaly",
            vec![
                ("kind", Field::Text(self.kind.clone())),
                ("subject", Field::Id(self.subject)),
                ("other", Field::id(self.other)),
                ("message", Field::Text(self.message.clone())),
            ],
        )
    }
}
