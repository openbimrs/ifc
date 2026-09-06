//! `IfcOrganization` and `IfcOrganizationRelationship` — bounded IFC4
//! organization projections.

use ifc_model::EntityId;

use crate::actor::role::ActorRole;
use crate::error::ResourceResult;
use crate::view::{Record, ResourceView};

/// A borrowed, schema-resolved `IfcOrganization` projection.
#[derive(Debug, Clone, Copy)]
pub struct Organization<'m, 's> {
    record: Record<'m, 's>,
}

impl<'m, 's> Organization<'m, 's> {
    pub(crate) fn from_record(record: Record<'m, 's>) -> ResourceResult<Self> {
        Ok(Self { record })
    }

    #[must_use]
    pub fn id(&self) -> EntityId {
        self.record.id
    }

    pub fn identification(&self) -> ResourceResult<Option<&'m str>> {
        self.record.optional_text("Identification")
    }

    pub fn name(&self) -> ResourceResult<&'m str> {
        self.record.required_text("Name")
    }

    pub fn description(&self) -> ResourceResult<Option<&'m str>> {
        self.record.optional_text("Description")
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

/// Borrowed `IfcOrganizationRelationship` projection: a named relationship
/// between one relating organization and one or more related organizations.
#[derive(Debug, Clone, Copy)]
pub struct OrganizationRelationship<'m, 's> {
    record: Record<'m, 's>,
}

impl<'m, 's> OrganizationRelationship<'m, 's> {
    pub(crate) fn from_record(record: Record<'m, 's>) -> ResourceResult<Self> {
        Ok(Self { record })
    }

    #[must_use]
    pub fn id(&self) -> EntityId {
        self.record.id
    }

    pub fn name(&self) -> ResourceResult<Option<&'m str>> {
        self.record.optional_text("Name")
    }

    pub fn description(&self) -> ResourceResult<Option<&'m str>> {
        self.record.optional_text("Description")
    }

    pub fn relating_organization(&self) -> ResourceResult<EntityId> {
        self.record
            .required_ref("RelatingOrganization", "IfcOrganization")
    }

    pub fn related_organizations(&self) -> ResourceResult<Vec<EntityId>> {
        self.record
            .refs("RelatedOrganizations", "IfcOrganization", 1, false, true)
    }
}

impl<'m, 's> ResourceView<'m, 's> {
    pub fn organization_relationship(
        &self,
        id: EntityId,
    ) -> ResourceResult<OrganizationRelationship<'m, 's>> {
        OrganizationRelationship::from_record(self.record(id, "IfcOrganizationRelationship")?)
    }
}
