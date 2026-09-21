//! Facilities and facility parts: the non-building spatial containers.
//!
//! `IfcBuilding` is one facility among several. IFC4X3 added bridges,
//! roads, railways and marine facilities as siblings, each with its own
//! `PredefinedType` enum, and a parallel family of parts.
//!
//! Two slot layouts, and the difference is not cosmetic:
//!
//! * A facility carries an optional `PredefinedType` at slot 9.
//! * A part carries a **mandatory** `UsageType` at 9 and pushes
//!   `PredefinedType` to 10.
//!
//! Writing a part as though it were a facility puts the usage token in
//! the predefined slot, which parses and means something else.

mod table;

use ifc_model::guid::Guid;
use ifc_model::{Entity, EntityId, Transaction, Value};

pub use table::{ALL, IFCBRIDGE, IFCBRIDGEPART, IFCFACILITY, IFCFACILITYPARTCOMMON};
pub use table::{IFCMARINEFACILITY, IFCMARINEPART, IFCRAILWAY, IFCRAILWAYPART};
pub use table::{IFCROAD, IFCROADPART};

/// One facility class and the slots that distinguish it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Facility {
    /// STEP type name, upper-case as stored.
    pub type_name: &'static str,
    /// Attribute count this class declares.
    pub arity: usize,
    /// Slot holding `PredefinedType`, when the class declares one.
    pub predefined_slot: Option<usize>,
    /// Slot holding the mandatory `UsageType`; parts only.
    pub usage_slot: Option<usize>,
    /// Permitted `PredefinedType` tokens for this exact class.
    pub members: &'static [&'static str],
}

impl Facility {
    /// Is this a facility part rather than a facility?
    ///
    /// Parts are the classes carrying a mandatory `UsageType`.
    #[must_use]
    pub const fn is_part(&self) -> bool {
        self.usage_slot.is_some()
    }
}

/// Why a facility was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FacilityError {
    /// `GlobalId` is absent or not a 22-character IFC GUID.
    MalformedGuid {
        /// The class being authored.
        entity: &'static str,
        /// What the caller offered.
        offered: String,
    },
    /// The token is not in this class's own enum.
    UnknownToken {
        /// The class being authored.
        entity: &'static str,
        /// Which attribute rejected it.
        attribute: &'static str,
        /// The offered token.
        offered: String,
    },
    /// A class with no `PredefinedType` was offered one.
    NoPredefinedType {
        /// The class being authored.
        entity: &'static str,
    },
    /// `UsageType` is mandatory on a part and was not supplied.
    MissingUsageType {
        /// The class being authored.
        entity: &'static str,
    },
    /// `UsageType` was supplied for a class that declares none.
    UnexpectedUsageType {
        /// The class being authored.
        entity: &'static str,
    },
    /// `USERDEFINED` was chosen without the name it promises.
    UserDefinedWithoutObjectType {
        /// The class being authored.
        entity: &'static str,
        /// Which attribute was `USERDEFINED`.
        attribute: &'static str,
    },
}

impl core::fmt::Display for FacilityError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::MalformedGuid { entity, offered } => {
                write!(f, "{entity}: GlobalId {offered:?} is not an IFC GUID")
            }
            Self::UnknownToken {
                entity,
                attribute,
                offered,
            } => {
                write!(f, "{entity}: {offered:?} is not a {attribute} of {entity}")
            }
            Self::NoPredefinedType { entity } => {
                write!(f, "{entity} declares no PredefinedType")
            }
            Self::MissingUsageType { entity } => {
                write!(f, "{entity}: UsageType is mandatory on a facility part")
            }
            Self::UnexpectedUsageType { entity } => {
                write!(f, "{entity} declares no UsageType")
            }
            Self::UserDefinedWithoutObjectType { entity, attribute } => {
                write!(
                    f,
                    "{entity}: {attribute} is USERDEFINED without an ObjectType naming it"
                )
            }
        }
    }
}

impl std::error::Error for FacilityError {}

/// Result of authoring a facility.
pub type FacilityResult<T> = Result<T, FacilityError>;

/// Attributes a facility shares with every spatial container.
#[derive(Debug, Clone, Copy, Default)]
pub struct FacilityDraft<'a> {
    /// `IfcRoot.Name`, slot 2.
    pub name: Option<&'a str>,
    /// `IfcRoot.Description`, slot 3.
    pub description: Option<&'a str>,
    /// `IfcObject.ObjectType`, slot 4. Names a `USERDEFINED` token.
    pub object_type: Option<&'a str>,
    /// `IfcProduct.ObjectPlacement`, slot 5.
    pub placement: Option<EntityId>,
    /// `IfcSpatialElement.LongName`, slot 7.
    pub long_name: Option<&'a str>,
    /// `IfcSpatialStructureElement.CompositionType`, slot 8.
    pub composition: Option<&'a str>,
    /// `UsageType`, slot 9. Mandatory on a part, refused otherwise.
    pub usage: Option<&'a str>,
}

const USAGE_TOKENS: &[&str] = &[
    "LATERAL",
    "LONGITUDINAL",
    "REGION",
    "VERTICAL",
    "USERDEFINED",
    "NOTDEFINED",
];

fn names_itself(draft: &FacilityDraft<'_>) -> bool {
    draft.object_type.is_some_and(|s| !s.trim().is_empty())
}

/// Stage a facility or facility part.
///
/// `kind` carries its own slot layout and its own enum, so a part and a
/// facility cannot be confused for one another.
///
/// # Errors
///
/// Refuses a malformed GlobalId; a `PredefinedType` or `UsageType` token
/// outside this exact class's enum; a `PredefinedType` on a class that
/// declares none; a missing `UsageType` on a part; a `UsageType` on a
/// class that declares none; and `USERDEFINED` in either attribute
/// without an `ObjectType` naming it.
pub fn create_facility(
    tx: &mut Transaction,
    kind: Facility,
    global_id: &str,
    predefined_type: Option<&str>,
    draft: FacilityDraft<'_>,
) -> FacilityResult<EntityId> {
    let entity = kind.type_name;
    if Guid::parse(global_id).is_none() {
        return Err(FacilityError::MalformedGuid {
            entity,
            offered: global_id.to_owned(),
        });
    }

    let mut attributes = vec![Value::Null; kind.arity];
    attributes[0] = Value::Text(global_id.into());
    attributes[2] = text(draft.name);
    attributes[3] = text(draft.description);
    attributes[4] = text(draft.object_type);
    attributes[5] = draft.placement.map_or(Value::Null, Value::Ref);
    attributes[7] = text(draft.long_name);
    attributes[8] = draft
        .composition
        .map_or(Value::Null, |t| Value::Enum(t.into()));

    match (kind.usage_slot, draft.usage) {
        (Some(slot), Some(token)) => {
            if !USAGE_TOKENS.contains(&token) {
                return Err(FacilityError::UnknownToken {
                    entity,
                    attribute: "UsageType",
                    offered: token.to_owned(),
                });
            }
            if token == "USERDEFINED" && !names_itself(&draft) {
                return Err(FacilityError::UserDefinedWithoutObjectType {
                    entity,
                    attribute: "UsageType",
                });
            }
            attributes[slot] = Value::Enum(token.into());
        }
        (Some(_), None) => return Err(FacilityError::MissingUsageType { entity }),
        (None, Some(_)) => return Err(FacilityError::UnexpectedUsageType { entity }),
        (None, None) => {}
    }

    match (kind.predefined_slot, predefined_type) {
        (Some(slot), Some(token)) => {
            if !kind.members.contains(&token) {
                return Err(FacilityError::UnknownToken {
                    entity,
                    attribute: "PredefinedType",
                    offered: token.to_owned(),
                });
            }
            if token == "USERDEFINED" && !names_itself(&draft) {
                return Err(FacilityError::UserDefinedWithoutObjectType {
                    entity,
                    attribute: "PredefinedType",
                });
            }
            attributes[slot] = Value::Enum(token.into());
        }
        (None, Some(_)) => return Err(FacilityError::NoPredefinedType { entity }),
        (Some(_), None) | (None, None) => {}
    }

    Ok(tx.create(Entity::new(entity, attributes)))
}

fn text(value: Option<&str>) -> Value {
    value.map_or(Value::Null, |t| Value::Text(t.into()))
}
