//! Staging actors, occupants, and assets.
//!
//! # Who a record names
//!
//! `IfcActor` names a party playing a role on the project; an
//! `IfcOccupant` is that party in a tenancy role. Both point at an
//! `IfcActorSelect`, which is the party itself: a person, an
//! organization, or a person acting for one. An actor whose
//! `TheActor` does not resolve names nobody, and the assignment
//! relationships that hang off it inherit that emptiness.
//!
//! `IfcAsset` is a group with a monetary tail: three cost values, a
//! depreciated value, an owner, a user, and a responsible person. It
//! is the accounting view of things already modelled elsewhere, so
//! every attribute past the group slots is optional.

use ifc_model::{EntityId, Value};

use crate::author::editor::{build_entity, text, validate_enum, ResourceEditor};
use crate::error::{ResourceError, ResourceResult};

/// The three `IfcActorSelect` members.
///
/// Named rather than inlined because both the actor writer and the
/// asset writer point at the same select, and a mismatch between
/// them would accept a party in one place and refuse it in the other.
const ACTOR_SELECT: &[&str] = &["IfcOrganization", "IfcPerson", "IfcPersonAndOrganization"];

/// Attributes of an `IfcActor` or `IfcOccupant` beyond the party.
#[derive(Debug, Clone, Copy)]
pub struct ActorDraft<'a> {
    /// `GlobalId`, a compressed IFC GUID.
    pub global_id: &'a str,
    /// `TheActor`: the party this record speaks for.
    ///
    /// Required by the schema and not defaultable: a record naming
    /// nobody identifies no party.
    pub the_actor: EntityId,
    /// `Name`.
    pub name: Option<&'a str>,
    /// `Description`.
    pub description: Option<&'a str>,
    /// `ObjectType`. Required when `predefined_type` is `USERDEFINED`.
    pub object_type: Option<&'a str>,
    /// `PredefinedType`, an `IfcOccupantTypeEnum` token.
    ///
    /// `IfcActor` declares no predefined type; supplying one for a
    /// plain actor is refused rather than dropped.
    pub predefined_type: Option<&'a str>,
}

/// Attributes of an `IfcAsset`.
///
/// Every attribute past the group slots is optional: an asset is the
/// accounting view of things modelled elsewhere, and a register may
/// know an item's owner long before it knows its depreciated value.
#[derive(Debug, Clone, Copy, Default)]
pub struct AssetDraft<'a> {
    /// `GlobalId`, a compressed IFC GUID.
    pub global_id: &'a str,
    /// `Name`.
    pub name: Option<&'a str>,
    /// `Description`.
    pub description: Option<&'a str>,
    /// `ObjectType`.
    pub object_type: Option<&'a str>,
    /// `Identification`: the asset register's own number.
    pub identification: Option<&'a str>,
    /// `OriginalValue`, an `IfcCostValue`.
    pub original_value: Option<EntityId>,
    /// `CurrentValue`, an `IfcCostValue`.
    pub current_value: Option<EntityId>,
    /// `TotalReplacementCost`, an `IfcCostValue`.
    pub total_replacement_cost: Option<EntityId>,
    /// `Owner`, an `IfcActorSelect`.
    pub owner: Option<EntityId>,
    /// `User`, an `IfcActorSelect`.
    pub user: Option<EntityId>,
    /// `ResponsiblePerson`, an `IfcPerson`.
    pub responsible_person: Option<EntityId>,
    /// `IncorporationDate`, an `IfcDate` in ISO 8601 form.
    pub incorporation_date: Option<&'a str>,
    /// `DepreciatedValue`, an `IfcCostValue`.
    pub depreciated_value: Option<EntityId>,
}

impl ResourceEditor<'_> {
    /// Stage an `IfcActor`, or an `IfcOccupant` when a predefined
    /// type is given.
    ///
    /// `the_actor` must resolve to an `IfcActorSelect` member. A
    /// record naming nobody identifies no party, so the reference is
    /// checked before staging.
    ///
    /// # Errors
    ///
    /// Refuses a malformed or duplicate GlobalId, a `the_actor` outside
    /// `IfcActorSelect`, a token outside `IfcOccupantTypeEnum`, a
    /// predefined type on a plain `IfcActor`, and `USERDEFINED`
    /// without `ObjectType` (WR31).
    pub fn create_actor(&mut self, draft: ActorDraft<'_>) -> ResourceResult<EntityId> {
        let occupant = draft.predefined_type.is_some();
        let entity_type = if occupant { "IfcOccupant" } else { "IfcActor" };
        self.validate_new_global_id(draft.global_id)?;
        self.check_reference_select(
            draft.the_actor,
            "TheActor",
            "IfcActorSelect",
            ACTOR_SELECT,
            draft.the_actor,
        )?;
        if let Some(value) = draft.predefined_type {
            validate_enum(self.schema, entity_type, "PredefinedType", value)?;
            if value == "USERDEFINED"
                && draft
                    .object_type
                    .is_none_or(|value| value.trim().is_empty())
            {
                return Err(ResourceError::SemanticViolation {
                    entity: None,
                    rule: "USERDEFINED_REQUIRES_OBJECT_TYPE",
                });
            }
        }
        let entity = build_entity(
            self.schema,
            entity_type,
            &[
                ("GlobalId", Some(text(draft.global_id))),
                ("Name", draft.name.map(text)),
                ("Description", draft.description.map(text)),
                ("ObjectType", draft.object_type.map(text)),
                ("TheActor", Some(Value::Ref(draft.the_actor))),
                (
                    "PredefinedType",
                    draft.predefined_type.map(|t| Value::Enum(t.into())),
                ),
            ],
        )?;
        self.commit_create(entity)
    }

    /// Stage an `IfcAsset`.
    ///
    /// The three value slots are `IfcCostValue` references and the
    /// two party slots are `IfcActorSelect`; each is checked before
    /// staging so an asset cannot claim a value or an owner that the
    /// model does not hold.
    ///
    /// # Errors
    ///
    /// Refuses a malformed or duplicate GlobalId, a value reference
    /// that is not an `IfcCostValue`, a party outside `IfcActorSelect`,
    /// and a responsible person that is not an `IfcPerson`.
    pub fn create_asset(&mut self, draft: AssetDraft<'_>) -> ResourceResult<EntityId> {
        self.validate_new_global_id(draft.global_id)?;
        for (attribute, target) in [
            ("OriginalValue", draft.original_value),
            ("CurrentValue", draft.current_value),
            ("TotalReplacementCost", draft.total_replacement_cost),
            ("DepreciatedValue", draft.depreciated_value),
        ] {
            if let Some(target) = target {
                self.check_reference(target, attribute, "IfcCostValue", target)?;
            }
        }
        for (attribute, target) in [("Owner", draft.owner), ("User", draft.user)] {
            if let Some(target) = target {
                self.check_reference_select(
                    target,
                    attribute,
                    "IfcActorSelect",
                    ACTOR_SELECT,
                    target,
                )?;
            }
        }
        if let Some(person) = draft.responsible_person {
            self.check_reference(person, "ResponsiblePerson", "IfcPerson", person)?;
        }
        let entity = build_entity(
            self.schema,
            "IfcAsset",
            &[
                ("GlobalId", Some(text(draft.global_id))),
                ("Name", draft.name.map(text)),
                ("Description", draft.description.map(text)),
                ("ObjectType", draft.object_type.map(text)),
                ("Identification", draft.identification.map(text)),
                ("OriginalValue", draft.original_value.map(Value::Ref)),
                ("CurrentValue", draft.current_value.map(Value::Ref)),
                (
                    "TotalReplacementCost",
                    draft.total_replacement_cost.map(Value::Ref),
                ),
                ("Owner", draft.owner.map(Value::Ref)),
                ("User", draft.user.map(Value::Ref)),
                (
                    "ResponsiblePerson",
                    draft.responsible_person.map(Value::Ref),
                ),
                ("IncorporationDate", draft.incorporation_date.map(text)),
                ("DepreciatedValue", draft.depreciated_value.map(Value::Ref)),
            ],
        )?;
        self.commit_create(entity)
    }
}
