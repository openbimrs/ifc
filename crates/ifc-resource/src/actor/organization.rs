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

    /// The entity id of the projected `IfcOrganization`.
    #[must_use]
    pub fn id(&self) -> EntityId {
        self.record.id
    }

    /// The `Identification` attribute, when authored.
    pub fn identification(&self) -> ResourceResult<Option<&'m str>> {
        self.record.optional_text("Identification")
    }

    /// The `Name` attribute.
    pub fn name(&self) -> ResourceResult<&'m str> {
        self.record.required_text("Name")
    }

    /// The `Description` attribute, when authored.
    pub fn description(&self) -> ResourceResult<Option<&'m str>> {
        self.record.optional_text("Description")
    }

    /// The `Roles` attribute: entity references, when authored.
    pub fn role_ids(&self) -> ResourceResult<Vec<EntityId>> {
        self.record.refs("Roles", "IfcActorRole", 1, true, false)
    }

    /// The `Roles` attribute, resolved to projected `IfcActorRole` values.
    pub fn roles(&self) -> ResourceResult<Vec<ActorRole<'m, 's>>> {
        self.role_ids()?
            .into_iter()
            .map(|id| {
                Record::new(self.record.model, self.record.schema, id, "IfcActorRole")
                    .and_then(ActorRole::from_record)
            })
            .collect()
    }

    /// The `Addresses` attribute, when authored.
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

    /// The entity id of the projected `IfcOrganizationRelationship`.
    #[must_use]
    pub fn id(&self) -> EntityId {
        self.record.id
    }

    /// The `Name` attribute, when authored.
    pub fn name(&self) -> ResourceResult<Option<&'m str>> {
        self.record.optional_text("Name")
    }

    /// The `Description` attribute, when authored.
    pub fn description(&self) -> ResourceResult<Option<&'m str>> {
        self.record.optional_text("Description")
    }

    /// The `RelatingOrganization` attribute.
    pub fn relating_organization(&self) -> ResourceResult<EntityId> {
        self.record
            .required_ref("RelatingOrganization", "IfcOrganization")
    }

    /// The `RelatedOrganizations` attribute.
    pub fn related_organizations(&self) -> ResourceResult<Vec<EntityId>> {
        self.record
            .refs("RelatedOrganizations", "IfcOrganization", 1, false, true)
    }
}

impl<'m, 's> ResourceView<'m, 's> {
    /// Projects an `IfcOrganizationRelationship` by entity id.
    pub fn organization_relationship(
        &self,
        id: EntityId,
    ) -> ResourceResult<OrganizationRelationship<'m, 's>> {
        OrganizationRelationship::from_record(self.record(id, "IfcOrganizationRelationship")?)
    }
}
