//! The three named system classifications.
//!
//! # Why these are not `create_system`
//!
//! `IfcSystem` is the unclassified group. These three add a
//! `PredefinedType` naming what kind of system it is, and they do not
//! agree on where it sits: `IfcBuildingSystem` and `IfcBuiltSystem`
//! put it at slot 5 with `LongName` at 6, while
//! `IfcDistributionCircuit` puts `LongName` at 5 and the type at 6.
//! Writing one layout for all three files a circuit's name into its
//! classification slot, producing a system whose kind is its name.
//!
//! # CorrectPredefinedType
//!
//! `IfcBuildingSystem` and `IfcBuiltSystem` both state:
//!
//! ```text
//! NOT(EXISTS(PredefinedType)) OR (PredefinedType <> USERDEFINED) OR
//! ((PredefinedType = USERDEFINED) AND EXISTS(SELF\\IfcObject.ObjectType))
//! ```
//!
//! `USERDEFINED` says the enum has no token for this kind and the name
//! is given as `ObjectType`. Without it the record asserts a name
//! exists and then withholds it.

use ifc_model::guid::Guid;
use ifc_model::{Entity, EntityId, Transaction, Value};
use ifc_schema::Schema;

use super::{invalid, SystemAuthoringResult};

/// `IfcBuildingSystemTypeEnum`, IFC4 and IFC4X3.
const BUILDING: &[&str] = &[
    "FENESTRATION",
    "FOUNDATION",
    "LOADBEARING",
    "OUTERSHELL",
    "SHADING",
    "TRANSPORT",
    "USERDEFINED",
    "NOTDEFINED",
];

/// `IfcBuiltSystemTypeEnum`, IFC4X3 only.
///
/// A superset of the building-system tokens: IFC4X3 renamed the
/// entity and widened the list to cover civil works.
const BUILT: &[&str] = &[
    "EROSIONPREVENTION",
    "FENESTRATION",
    "FOUNDATION",
    "LOADBEARING",
    "MOORING",
    "OUTERSHELL",
    "PRESTRESSING",
    "RAILWAYLINE",
    "RAILWAYTRACK",
    "REINFORCING",
    "SHADING",
    "TRACKCIRCUIT",
    "TRANSPORT",
    "USERDEFINED",
    "NOTDEFINED",
];

/// `IfcDistributionSystemEnum`.
///
/// The IFC4X3 list, a superset of IFC4's. Accepting the superset on
/// both schemas would let an IFC4 file name a system kind IFC4 does
/// not declare, so the schema in hand selects the slice.
const DISTRIBUTION_X3: &[&str] = &[
    "AIRCONDITIONING",
    "AUDIOVISUAL",
    "CATENARY_SYSTEM",
    "CHEMICAL",
    "CHILLEDWATER",
    "COMMUNICATION",
    "COMPRESSEDAIR",
    "CONDENSERWATER",
    "CONTROL",
    "CONVEYING",
    "DATA",
    "DISPOSAL",
    "DOMESTICCOLDWATER",
    "DOMESTICHOTWATER",
    "DRAINAGE",
    "EARTHING",
    "ELECTRICAL",
    "ELECTROACOUSTIC",
    "EXHAUST",
    "FIREPROTECTION",
    "FIXEDTRANSMISSIONNETWORK",
    "FUEL",
    "GAS",
    "HAZARDOUS",
    "HEATING",
    "LIGHTING",
    "LIGHTNINGPROTECTION",
    "MOBILENETWORK",
    "MONITORINGSYSTEM",
    "MUNICIPALSOLIDWASTE",
    "OIL",
    "OPERATIONAL",
    "OPERATIONALTELEPHONYSYSTEM",
    "OVERHEAD_CONTACTLINE_SYSTEM",
    "POWERGENERATION",
    "RAINWATER",
    "REFRIGERATION",
    "RETURN_CIRCUIT",
    "SECURITY",
    "SEWAGE",
    "SIGNAL",
    "STORMWATER",
    "TELEPHONE",
    "TV",
    "VACUUM",
    "VENT",
    "VENTILATION",
    "WASTEWATER",
    "WATERSUPPLY",
    "USERDEFINED",
    "NOTDEFINED",
];

/// The IFC4 subset of [`DISTRIBUTION_X3`].
const DISTRIBUTION_IFC4: &[&str] = &[
    "AIRCONDITIONING",
    "AUDIOVISUAL",
    "CHEMICAL",
    "CHILLEDWATER",
    "COMMUNICATION",
    "COMPRESSEDAIR",
    "CONDENSERWATER",
    "CONTROL",
    "CONVEYING",
    "DATA",
    "DISPOSAL",
    "DOMESTICCOLDWATER",
    "DOMESTICHOTWATER",
    "DRAINAGE",
    "EARTHING",
    "ELECTRICAL",
    "ELECTROACOUSTIC",
    "EXHAUST",
    "FIREPROTECTION",
    "FUEL",
    "GAS",
    "HAZARDOUS",
    "HEATING",
    "LIGHTING",
    "LIGHTNINGPROTECTION",
    "MUNICIPALSOLIDWASTE",
    "OIL",
    "OPERATIONAL",
    "POWERGENERATION",
    "RAINWATER",
    "REFRIGERATION",
    "SECURITY",
    "SEWAGE",
    "SIGNAL",
    "STORMWATER",
    "TELEPHONE",
    "TV",
    "VACUUM",
    "VENT",
    "VENTILATION",
    "WASTEWATER",
    "WATERSUPPLY",
    "USERDEFINED",
    "NOTDEFINED",
];

/// Which classified system is being staged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SystemKind {
    /// `IfcBuildingSystem`. Superseded by [`SystemKind::Built`] in
    /// IFC4X3, which still declares it.
    Building,
    /// `IfcBuiltSystem`, IFC4X3 only.
    Built,
    /// `IfcDistributionCircuit`: a branch of a distribution system.
    DistributionCircuit,
    /// `IfcDistributionSystem`: the system a circuit branches from.
    DistributionSystem,
}

impl SystemKind {
    /// STEP type name, upper-case as the model stores it.
    const fn type_name(self) -> &'static str {
        match self {
            Self::Building => "IFCBUILDINGSYSTEM",
            Self::Built => "IFCBUILTSYSTEM",
            Self::DistributionCircuit => "IFCDISTRIBUTIONCIRCUIT",
            Self::DistributionSystem => "IFCDISTRIBUTIONSYSTEM",
        }
    }

    /// Tokens this entity's enum declares under `schema`.
    ///
    /// `IfcDistributionSystemEnum` is the only one that differs
    /// between the two shipped schemas.
    fn members(self, schema: &Schema) -> &'static [&'static str] {
        match self {
            Self::Building => BUILDING,
            Self::Built => BUILT,
            Self::DistributionCircuit | Self::DistributionSystem => {
                if schema.name().eq_ignore_ascii_case("IFC4") {
                    DISTRIBUTION_IFC4
                } else {
                    DISTRIBUTION_X3
                }
            }
        }
    }
    /// Slot holding `PredefinedType`.
    ///
    /// A circuit puts `LongName` first; the other two put the type
    /// first. This is the whole reason the three cannot share a path.
    const fn predefined_slot(self) -> usize {
        match self {
            Self::Building | Self::Built => 5,
            Self::DistributionCircuit | Self::DistributionSystem => 6,
        }
    }

    /// Slot holding `LongName`, the mirror of `predefined_slot`.
    const fn long_name_slot(self) -> usize {
        match self {
            Self::Building | Self::Built => 6,
            Self::DistributionCircuit | Self::DistributionSystem => 5,
        }
    }
}

/// Attributes of a classified system beyond `GlobalId` and `Name`.
#[derive(Debug, Clone, Copy, Default)]
pub struct ClassifiedSystemDraft<'a> {
    /// `Description`.
    pub description: Option<&'a str>,
    /// `ObjectType`. Required when `predefined_type` is `USERDEFINED`.
    pub object_type: Option<&'a str>,
    /// `PredefinedType`, a token of this entity's own enum.
    pub predefined_type: Option<&'a str>,
    /// `LongName`: the full name where `Name` is an abbreviation.
    pub long_name: Option<&'a str>,
}
/// Stage a classified system.
///
/// `predefined_type` must be a token the entity's own enum declares,
/// resolved against `schema`: `IfcDistributionSystemEnum` gained seven
/// members in IFC4X3, and accepting those on IFC4 would name a system
/// kind that schema does not have.
///
/// # Errors
///
/// Refuses a malformed GlobalId; an entity the schema does not declare
/// (`IfcBuiltSystem` on IFC4); a token outside the entity's enum for
/// the schema in hand; and `USERDEFINED` without `object_type`
/// (CorrectPredefinedType).
pub fn create_classified_system(
    tx: &mut Transaction,
    schema: &Schema,
    kind: SystemKind,
    global_id: &str,
    name: Option<&str>,
    draft: ClassifiedSystemDraft<'_>,
) -> SystemAuthoringResult<EntityId> {
    let entity = kind.type_name();
    if Guid::parse(global_id).is_none() {
        return Err(invalid(entity, "GlobalId", global_id));
    }

    // Arity comes from the schema, which also answers whether this
    // schema declares the entity at all.
    let declared = schema.attributes(entity);
    if declared.is_empty() {
        return Err(invalid(entity, "Entity", schema.name().to_owned()));
    }

    if let Some(token) = draft.predefined_type {
        if !kind.members(schema).contains(&token) {
            return Err(invalid(entity, "PredefinedType", token));
        }
        if token == "USERDEFINED" && blank(draft.object_type) {
            return Err(invalid(entity, "ObjectType", "required by USERDEFINED"));
        }
    }

    let mut attributes = vec![Value::Null; declared.len()];
    attributes[0] = Value::Text(global_id.into());
    attributes[2] = text(name);
    attributes[3] = text(draft.description);
    attributes[4] = text(draft.object_type);
    attributes[kind.predefined_slot()] = draft
        .predefined_type
        .map_or(Value::Null, |t| Value::Enum(t.into()));
    attributes[kind.long_name_slot()] = text(draft.long_name);
    Ok(tx.create(Entity::new(entity, attributes)))
}

fn text(value: Option<&str>) -> Value {
    value.map_or(Value::Null, |v| Value::Text(v.into()))
}

fn blank(value: Option<&str>) -> bool {
    value.is_none_or(|v| v.trim().is_empty())
}
