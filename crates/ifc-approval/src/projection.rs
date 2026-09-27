//! Strict borrowed projections and deterministic direct queries.

use ifc_model::guid::Guid;
use ifc_model::{Entity, EntityId};

use crate::view::{
    optional_ref, optional_text, required_ref, required_refs, required_text, validate_target,
    wrong, ApprovalView,
};
use crate::{ApprovalError, ApprovalResult};

const APPROVAL: &str = "IFCAPPROVAL";
const APPROVAL_REL: &str = "IFCAPPROVALRELATIONSHIP";
const RESOURCE_REL: &str = "IFCRESOURCEAPPROVALRELATIONSHIP";
const ASSIGNMENT: &str = "IFCRELASSOCIATESAPPROVAL";

macro_rules! projection {
    ($name:ident, $kind:expr) => {
        /// Strict borrowed IFC4 projection.
        #[derive(Debug, Clone, Copy)]
        pub struct $name<'m> {
            id: EntityId,
            entity: &'m Entity,
        }
        impl<'m> $name<'m> {
            /// Construct from an entity of the exact expected kind.
            pub fn try_new(id: EntityId, entity: &'m Entity) -> ApprovalResult<Self> {
                if entity.is_type($kind) {
                    Ok(Self { id, entity })
                } else {
                    Err(wrong($kind, entity))
                }
            }
            /// Stable model identifier.
            #[must_use]
            pub const fn id(self) -> EntityId {
                self.id
            }
        }
    };
}

projection!(Approval, APPROVAL);
projection!(ApprovalRelationship, APPROVAL_REL);
projection!(ResourceApprovalRelationship, RESOURCE_REL);
projection!(ApprovalAssignment, ASSIGNMENT);

impl<'m> Approval<'m> {
    /// Optional approval identifier.
    pub fn identifier(self) -> ApprovalResult<Option<&'m str>> {
        optional_text(APPROVAL, self.id, self.entity, 0, "Identifier")
    }
    /// Optional approval name.
    pub fn name(self) -> ApprovalResult<Option<&'m str>> {
        optional_text(APPROVAL, self.id, self.entity, 1, "Name")
    }
    /// Optional description.
    pub fn description(self) -> ApprovalResult<Option<&'m str>> {
        optional_text(APPROVAL, self.id, self.entity, 2, "Description")
    }
    /// Optional authored approval time.
    pub fn time_of_approval(self) -> ApprovalResult<Option<&'m str>> {
        optional_text(APPROVAL, self.id, self.entity, 3, "TimeOfApproval")
    }
    /// Optional status label.
    pub fn status(self) -> ApprovalResult<Option<&'m str>> {
        optional_text(APPROVAL, self.id, self.entity, 4, "Status")
    }
    /// Optional approval level label.
    pub fn level(self) -> ApprovalResult<Option<&'m str>> {
        optional_text(APPROVAL, self.id, self.entity, 5, "Level")
    }
    /// Optional qualifier text.
    pub fn qualifier(self) -> ApprovalResult<Option<&'m str>> {
        optional_text(APPROVAL, self.id, self.entity, 6, "Qualifier")
    }
    /// Optional requesting actor-select reference.
    pub fn requesting_approval(self) -> ApprovalResult<Option<EntityId>> {
        optional_ref(APPROVAL, self.id, self.entity, 7, "RequestingApproval")
    }
    /// Optional giving actor-select reference.
    pub fn giving_approval(self) -> ApprovalResult<Option<EntityId>> {
        optional_ref(APPROVAL, self.id, self.entity, 8, "GivingApproval")
    }

    fn validate(self, view: ApprovalView<'m>) -> ApprovalResult<Self> {
        if self.identifier()?.is_none() && self.name()?.is_none() {
            return Err(ApprovalError::Semantic {
                entity: APPROVAL,
                id: self.id,
                rule: "HasIdentifierOrName",
                detail: "Identifier and Name are both absent".into(),
            });
        }
        for (attribute, target) in [
            ("RequestingApproval", self.requesting_approval()?),
            ("GivingApproval", self.giving_approval()?),
        ] {
            if let Some(target) = target {
                validate_target(
                    view.model(),
                    APPROVAL,
                    self.id,
                    attribute,
                    target,
                    "IfcActorSelect",
                )?;
            }
        }
        Ok(self)
    }
}

impl<'m> ApprovalRelationship<'m> {
    /// Optional relationship name.
    pub fn name(self) -> ApprovalResult<Option<&'m str>> {
        optional_text(APPROVAL_REL, self.id, self.entity, 0, "Name")
    }
    /// Optional relationship description.
    pub fn description(self) -> ApprovalResult<Option<&'m str>> {
        optional_text(APPROVAL_REL, self.id, self.entity, 1, "Description")
    }
    /// Relating approval.
    pub fn relating_approval(self) -> ApprovalResult<EntityId> {
        required_ref(APPROVAL_REL, self.id, self.entity, 2, "RelatingApproval")
    }
    /// Non-empty unique related approvals.
    pub fn related_approvals(self) -> ApprovalResult<Vec<EntityId>> {
        required_refs(APPROVAL_REL, self.id, self.entity, 3, "RelatedApprovals")
    }
    fn validate(self, view: ApprovalView<'m>) -> ApprovalResult<Self> {
        let relating = self.relating_approval()?;
        validate_target(
            view.model(),
            APPROVAL_REL,
            self.id,
            "RelatingApproval",
            relating,
            APPROVAL,
        )?;
        for target in self.related_approvals()? {
            if target == relating {
                return Err(ApprovalError::Semantic {
                    entity: APPROVAL_REL,
                    id: self.id,
                    rule: "NoSelfRelationship",
                    detail: format!("approval {target} appears on both ends"),
                });
            }
            validate_target(
                view.model(),
                APPROVAL_REL,
                self.id,
                "RelatedApprovals",
                target,
                APPROVAL,
            )?;
        }
        Ok(self)
    }
}

impl<'m> ResourceApprovalRelationship<'m> {
    /// Optional relationship name.
    pub fn name(self) -> ApprovalResult<Option<&'m str>> {
        optional_text(RESOURCE_REL, self.id, self.entity, 0, "Name")
    }
    /// Optional relationship description.
    pub fn description(self) -> ApprovalResult<Option<&'m str>> {
        optional_text(RESOURCE_REL, self.id, self.entity, 1, "Description")
    }
    /// Non-empty unique resource-select targets.
    pub fn related_resources(self) -> ApprovalResult<Vec<EntityId>> {
        required_refs(
            RESOURCE_REL,
            self.id,
            self.entity,
            2,
            "RelatedResourceObjects",
        )
    }
    /// Approval governing the resources.
    pub fn relating_approval(self) -> ApprovalResult<EntityId> {
        required_ref(RESOURCE_REL, self.id, self.entity, 3, "RelatingApproval")
    }
    fn validate(self, view: ApprovalView<'m>) -> ApprovalResult<Self> {
        validate_target(
            view.model(),
            RESOURCE_REL,
            self.id,
            "RelatingApproval",
            self.relating_approval()?,
            APPROVAL,
        )?;
        for target in self.related_resources()? {
            validate_target(
                view.model(),
                RESOURCE_REL,
                self.id,
                "RelatedResourceObjects",
                target,
                "IfcResourceObjectSelect",
            )?;
        }
        Ok(self)
    }
}

impl<'m> ApprovalAssignment<'m> {
    /// Root GlobalId.
    pub fn global_id(self) -> ApprovalResult<&'m str> {
        required_text(ASSIGNMENT, self.id, self.entity, 0, "GlobalId")
    }
    /// Optional relationship name.
    pub fn name(self) -> ApprovalResult<Option<&'m str>> {
        optional_text(ASSIGNMENT, self.id, self.entity, 2, "Name")
    }
    /// Optional relationship description.
    pub fn description(self) -> ApprovalResult<Option<&'m str>> {
        optional_text(ASSIGNMENT, self.id, self.entity, 3, "Description")
    }
    /// Related definition-select targets.
    pub fn related_objects(self) -> ApprovalResult<Vec<EntityId>> {
        required_refs(ASSIGNMENT, self.id, self.entity, 4, "RelatedObjects")
    }
    /// Relating approval.
    pub fn relating_approval(self) -> ApprovalResult<EntityId> {
        required_ref(ASSIGNMENT, self.id, self.entity, 5, "RelatingApproval")
    }
    fn validate(self, view: ApprovalView<'m>) -> ApprovalResult<Self> {
        if Guid::parse(self.global_id()?).is_none() {
            return Err(ApprovalError::InvalidValue {
                entity: ASSIGNMENT,
                id: self.id,
                attribute: "GlobalId",
                value: self.global_id()?.into(),
            });
        }
        validate_target(
            view.model(),
            ASSIGNMENT,
            self.id,
            "RelatingApproval",
            self.relating_approval()?,
            APPROVAL,
        )?;
        for target in self.related_objects()? {
            validate_target(
                view.model(),
                ASSIGNMENT,
                self.id,
                "RelatedObjects",
                target,
                "IfcDefinitionSelect",
            )?;
        }
        Ok(self)
    }
}

impl<'m> ApprovalView<'m> {
    /// Strictly project one approval.
    pub fn approval(self, id: EntityId) -> ApprovalResult<Approval<'m>> {
        let entity = self
            .model()
            .get(id)
            .ok_or(ApprovalError::UnknownEntity { id })?;
        Approval::try_new(id, entity)?.validate(self)
    }
    /// Strictly project one approval-to-approval relationship.
    pub fn approval_relationship(self, id: EntityId) -> ApprovalResult<ApprovalRelationship<'m>> {
        let entity = self
            .model()
            .get(id)
            .ok_or(ApprovalError::UnknownEntity { id })?;
        ApprovalRelationship::try_new(id, entity)?.validate(self)
    }
    /// Strictly project one resource approval relationship.
    pub fn resource_approval_relationship(
        self,
        id: EntityId,
    ) -> ApprovalResult<ResourceApprovalRelationship<'m>> {
        let entity = self
            .model()
            .get(id)
            .ok_or(ApprovalError::UnknownEntity { id })?;
        ResourceApprovalRelationship::try_new(id, entity)?.validate(self)
    }
    /// Strictly project one rooted object approval association.
    pub fn approval_assignment(self, id: EntityId) -> ApprovalResult<ApprovalAssignment<'m>> {
        let entity = self
            .model()
            .get(id)
            .ok_or(ApprovalError::UnknownEntity { id })?;
        ApprovalAssignment::try_new(id, entity)?.validate(self)
    }

    /// Resource-select IDs directly governed by an approval.
    pub fn resources_approved_by(self, approval: EntityId) -> ApprovalResult<Vec<EntityId>> {
        self.approval(approval)?;
        let mut out = Vec::new();
        for (id, entity) in self.model().of_type(RESOURCE_REL) {
            let relationship = ResourceApprovalRelationship { id, entity };
            if relationship.relating_approval()? == approval {
                out.extend(relationship.validate(self)?.related_resources()?);
            }
        }
        Ok(out)
    }

    /// Definition-select IDs directly associated with an approval.
    pub fn objects_approved_by(self, approval: EntityId) -> ApprovalResult<Vec<EntityId>> {
        self.approval(approval)?;
        let mut out = Vec::new();
        for (id, entity) in self.model().of_type(ASSIGNMENT) {
            let assignment = ApprovalAssignment { id, entity };
            if assignment.relating_approval()? == approval {
                out.extend(assignment.validate(self)?.related_objects()?);
            }
        }
        Ok(out)
    }
}
