//! Actor capability: `IfcPerson`, `IfcOrganization`, `IfcPersonAndOrganization`,
//! `IfcActorRole`.
//!
//! ## Internal split
//!
//! - `person.rs`: people and identities.
//! - `organization.rs`: organizations/relationships.
//! - `role.rs`: actor roles.

mod organization;
mod person;
mod role;

pub use organization::{Organization, OrganizationRelationship};
pub use person::Person;
pub use role::ActorRole;

use ifc_model::EntityId;

use crate::error::ResourceResult;
use crate::view::{Record, ResourceView};

/// Borrowed `IfcPersonAndOrganization` projection.
#[derive(Debug, Clone, Copy)]
pub struct PersonAndOrganization<'m, 's> {
    record: Record<'m, 's>,
}

impl<'m, 's> PersonAndOrganization<'m, 's> {
    pub(crate) fn from_record(record: Record<'m, 's>) -> ResourceResult<Self> {
        Ok(Self { record })
    }

    /// The entity id of the projected `IfcPersonAndOrganization`.
    #[must_use]
    pub fn id(&self) -> EntityId {
        self.record.id
    }

    /// The `ThePerson` attribute.
    pub fn person(&self) -> ResourceResult<EntityId> {
        self.record.required_ref("ThePerson", "IfcPerson")
    }

    /// The `TheOrganization` attribute.
    pub fn organization(&self) -> ResourceResult<EntityId> {
        self.record
            .required_ref("TheOrganization", "IfcOrganization")
    }

    /// The `Roles` attribute, when authored.
    pub fn role_ids(&self) -> ResourceResult<Vec<EntityId>> {
        self.record.refs("Roles", "IfcActorRole", 1, true, false)
    }
}

impl<'m, 's> ResourceView<'m, 's> {
    /// Projects an `IfcPerson` by entity id.
    pub fn person(&self, id: EntityId) -> ResourceResult<Person<'m, 's>> {
        Person::from_record(self.record(id, "IfcPerson")?)
    }

    /// Projects an `IfcOrganization` by entity id.
    pub fn organization(&self, id: EntityId) -> ResourceResult<Organization<'m, 's>> {
        Organization::from_record(self.record(id, "IfcOrganization")?)
    }

    /// Projects an `IfcActorRole` by entity id.
    pub fn actor_role(&self, id: EntityId) -> ResourceResult<ActorRole<'m, 's>> {
        ActorRole::from_record(self.record(id, "IfcActorRole")?)
    }

    /// Projects an `IfcPersonAndOrganization` by entity id.
    pub fn person_and_organization(
        &self,
        id: EntityId,
    ) -> ResourceResult<PersonAndOrganization<'m, 's>> {
        PersonAndOrganization::from_record(self.record(id, "IfcPersonAndOrganization")?)
    }
}
