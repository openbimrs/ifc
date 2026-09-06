//! `IfcPerson` — bounded IFC4 person projection.

use ifc_model::EntityId;

use crate::actor::role::ActorRole;
use crate::error::{ResourceError, ResourceResult};
use crate::view::Record;

/// A borrowed, schema-resolved `IfcPerson` projection.
///
/// Enforces `IfcPerson.IdentifiablePerson` (at least one of `Identification`,
/// `FamilyName`, `GivenName` is present) and `IfcPerson.ValidSetOfNames`
/// (`MiddleNames` requires `FamilyName` or `GivenName`).
#[derive(Debug, Clone, Copy)]
pub struct Person<'m, 's> {
    record: Record<'m, 's>,
}

impl<'m, 's> Person<'m, 's> {
    pub(crate) fn from_record(record: Record<'m, 's>) -> ResourceResult<Self> {
        let person = Self { record };
        let identification = person.record.optional_text("Identification")?;
        let family_name = person.record.optional_text("FamilyName")?;
        let given_name = person.record.optional_text("GivenName")?;
        if identification.is_none() && family_name.is_none() && given_name.is_none() {
            return Err(ResourceError::SemanticViolation {
                entity: Some(person.record.id),
                rule:
                    "IfcPerson.IdentifiablePerson requires Identification, FamilyName, or GivenName",
            });
        }
        let middle_names = person.record.optional_text_list("MiddleNames", 1)?;
        if !middle_names.is_empty() && family_name.is_none() && given_name.is_none() {
            return Err(ResourceError::SemanticViolation {
                entity: Some(person.record.id),
                rule: "IfcPerson.ValidSetOfNames requires FamilyName or GivenName with MiddleNames",
            });
        }
        Ok(person)
    }

    #[must_use]
    pub fn id(&self) -> EntityId {
        self.record.id
    }

    pub fn identification(&self) -> ResourceResult<Option<&'m str>> {
        self.record.optional_text("Identification")
    }

    pub fn family_name(&self) -> ResourceResult<Option<&'m str>> {
        self.record.optional_text("FamilyName")
    }

    pub fn given_name(&self) -> ResourceResult<Option<&'m str>> {
        self.record.optional_text("GivenName")
    }

    pub fn middle_names(&self) -> ResourceResult<Vec<&'m str>> {
        self.record.optional_text_list("MiddleNames", 1)
    }

    pub fn prefix_titles(&self) -> ResourceResult<Vec<&'m str>> {
        self.record.optional_text_list("PrefixTitles", 1)
    }

    pub fn suffix_titles(&self) -> ResourceResult<Vec<&'m str>> {
        self.record.optional_text_list("SuffixTitles", 1)
    }

    pub fn role_ids(&self) -> ResourceResult<Vec<EntityId>> {
        self.record.refs("Roles", "IfcActorRole", 1, true, false)
    }

    pub fn roles(&self) -> ResourceResult<Vec<ActorRole<'m, 's>>> {
        self.role_ids()?
            .into_iter()
            .map(|id| {
                Record::new(self.record.model, self.record.schema, id, "IfcActorRole")
                    .and_then(ActorRole::from_record)
            })
            .collect()
    }

    pub fn address_ids(&self) -> ResourceResult<Vec<EntityId>> {
        self.record.refs("Addresses", "IfcAddress", 1, true, false)
    }
}
