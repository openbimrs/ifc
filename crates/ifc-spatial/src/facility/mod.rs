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
use ifc_model::{Entity, EntityId, Model, Transaction, Value};

use crate::authoring::release::stage;
use crate::authoring::SpatialAuthoringError;

pub use table::{ALL, IFCBRIDGE, IFCBRIDGEPART, IFCFACILITY, IFCFACILITYPARTCOMMON};
pub use table::{IFCMARINEFACILITY, IFCMARINEPART, IFCRAILWAY, IFCRAILWAYPART};
pub use table::{IFCROAD, IFCROADPART};

/// One facility class and the slots that distinguish it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
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
#[non_exhaustive]
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
    /// Refused against the model's declared release or its owner history,
    /// by [`create_facility_with_owner_history`] (#202).
    Authoring(SpatialAuthoringError),
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
            Self::Authoring(error) => error.fmt(f),
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
///
/// `#[non_exhaustive]`: build it with [`FacilityDraft::new`] and the
/// setters.
#[derive(Debug, Clone, Copy, Default)]
#[non_exhaustive]
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

impl<'a> FacilityDraft<'a> {
    /// An empty draft: every attribute unset.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set `IfcRoot.Name`.
    #[must_use]
    pub fn name(mut self, value: &'a str) -> Self {
        self.name = Some(value);
        self
    }

    /// Set `IfcRoot.Description`.
    #[must_use]
    pub fn description(mut self, value: &'a str) -> Self {
        self.description = Some(value);
        self
    }

    /// Set `IfcObject.ObjectType`, which names a `USERDEFINED` token.
    #[must_use]
    pub fn object_type(mut self, value: &'a str) -> Self {
        self.object_type = Some(value);
        self
    }

    /// Set `IfcProduct.ObjectPlacement`.
    #[must_use]
    pub fn placement(mut self, value: EntityId) -> Self {
        self.placement = Some(value);
        self
    }

    /// Set `LongName`.
    #[must_use]
    pub fn long_name(mut self, value: &'a str) -> Self {
        self.long_name = Some(value);
        self
    }

    /// Set `CompositionType`, an `IfcElementCompositionEnum` token.
    #[must_use]
    pub fn composition(mut self, value: &'a str) -> Self {
        self.composition = Some(value);
        self
    }

    /// Set `UsageType`; mandatory on a facility part, refused otherwise.
    #[must_use]
    pub fn usage(mut self, value: &'a str) -> Self {
        self.usage = Some(value);
        self
    }
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
/// IFC4X3 only, as are the classes: it writes the IFC4X3 layout with
/// `OwnerHistory` `$`. [`create_facility_with_owner_history`] binds the
/// model's declared release and takes an `IfcOwnerHistory`.
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
    check_facility(kind, global_id, predefined_type, &draft)?;
    let mut attributes = vec![Value::Null; kind.arity];
    attributes[0] = Value::Text(global_id.into());
    attributes[2] = text(draft.name);
    attributes[3] = text(draft.description);
    attributes[4] = text(draft.object_type);
    attributes[5] = draft.placement.map_or(Value::Null, Value::Ref);
    attributes[7] = text(draft.long_name);
    attributes[8] = enumeration(draft.composition);
    if let Some(slot) = kind.usage_slot {
        attributes[slot] = enumeration(draft.usage);
    }
    if let Some(slot) = kind.predefined_slot {
        attributes[slot] = enumeration(predefined_type);
    }
    Ok(tx.create(Entity::new(kind.type_name, attributes)))
}

/// [`create_facility`] in the model's declared release, with a
/// caller-supplied `IfcOwnerHistory` (#202).
///
/// The record is laid out by attribute name from the release's table.
/// Only IFC4X3 declares the facility classes, so an IFC2X3 or IFC4 model
/// is refused with `EntityNotInSchema`.
///
/// # Errors
///
/// Those of [`create_facility`], and [`FacilityError::Authoring`] wrapping
/// the release and owner-history refusals of
/// [`aggregate_with_owner_history`](crate::aggregate_with_owner_history).
/// Nothing is staged on an error.
pub fn create_facility_with_owner_history(
    tx: &mut Transaction,
    model: &Model,
    kind: Facility,
    global_id: &str,
    predefined_type: Option<&str>,
    draft: FacilityDraft<'_>,
    owner_history: EntityId,
) -> FacilityResult<EntityId> {
    check_facility(kind, global_id, predefined_type, &draft)?;
    let values = vec![
        ("GlobalId", Value::Text(global_id.into())),
        ("Name", text(draft.name)),
        ("Description", text(draft.description)),
        ("ObjectType", text(draft.object_type)),
        (
            "ObjectPlacement",
            draft.placement.map_or(Value::Null, Value::Ref),
        ),
        ("LongName", text(draft.long_name)),
        ("CompositionType", enumeration(draft.composition)),
        ("UsageType", enumeration(draft.usage)),
        ("PredefinedType", enumeration(predefined_type)),
    ];
    stage(tx, model, kind.type_name, values, Some(owner_history)).map_err(FacilityError::Authoring)
}

/// The checks of [`create_facility`].
fn check_facility(
    kind: Facility,
    global_id: &str,
    predefined_type: Option<&str>,
    draft: &FacilityDraft<'_>,
) -> FacilityResult<()> {
    let entity = kind.type_name;
    if Guid::parse(global_id).is_none() {
        return Err(FacilityError::MalformedGuid {
            entity,
            offered: global_id.to_owned(),
        });
    }
    match (kind.usage_slot, draft.usage) {
        (Some(_), Some(token)) => {
            if !USAGE_TOKENS.contains(&token) {
                return Err(FacilityError::UnknownToken {
                    entity,
                    attribute: "UsageType",
                    offered: token.to_owned(),
                });
            }
            if token == "USERDEFINED" && !names_itself(draft) {
                return Err(FacilityError::UserDefinedWithoutObjectType {
                    entity,
                    attribute: "UsageType",
                });
            }
        }
        (Some(_), None) => return Err(FacilityError::MissingUsageType { entity }),
        (None, Some(_)) => return Err(FacilityError::UnexpectedUsageType { entity }),
        (None, None) => {}
    }
    match (kind.predefined_slot, predefined_type) {
        (Some(_), Some(token)) => {
            if !kind.members.contains(&token) {
                return Err(FacilityError::UnknownToken {
                    entity,
                    attribute: "PredefinedType",
                    offered: token.to_owned(),
                });
            }
            if token == "USERDEFINED" && !names_itself(draft) {
                return Err(FacilityError::UserDefinedWithoutObjectType {
                    entity,
                    attribute: "PredefinedType",
                });
            }
        }
        (None, Some(_)) => return Err(FacilityError::NoPredefinedType { entity }),
        (Some(_), None) | (None, None) => {}
    }
    Ok(())
}

fn text(value: Option<&str>) -> Value {
    value.map_or(Value::Null, |t| Value::Text(t.into()))
}

fn enumeration(value: Option<&str>) -> Value {
    value.map_or(Value::Null, |t| Value::Enum(t.into()))
}
