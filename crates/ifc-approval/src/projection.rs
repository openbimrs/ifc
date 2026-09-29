//! Strict borrowed projections and deterministic direct queries, read by
//! attribute name in the model's declared release (#212).

use ifc_model::guid::Guid;
use ifc_model::{Entity, EntityId};
use ifc_schema::SchemaVersion;

use crate::release::Layout;
use crate::view::{wrong, ApprovalView, Record};
use crate::{ApprovalError, ApprovalResult};

const APPROVAL: &str = "IFCAPPROVAL";
const APPROVAL_REL: &str = "IFCAPPROVALRELATIONSHIP";
const RESOURCE_REL: &str = "IFCRESOURCEAPPROVALRELATIONSHIP";
const ASSIGNMENT: &str = "IFCRELASSOCIATESAPPROVAL";

macro_rules! projection {
    ($name:ident, $kind:expr) => {
        /// Strict borrowed projection, read against one release.
        #[derive(Debug, Clone, Copy)]
        pub struct $name<'m> {
            record: Record<'m>,
        }
        impl<'m> $name<'m> {
            /// Construct from an entity of the exact expected kind, read
            /// against `release`.
            ///
            /// [`ApprovalView`] binds the model's declared release; use this
            /// when the release is known some other way.
            ///
            /// # Errors
            ///
            /// `WrongEntityType` for another entity, `UnsupportedSchema`
            /// for a release this crate is not verified against (IFC4X1,
            /// IFC4X2), and `EntityNotInSchema` for an entity the release
            /// does not declare.
            pub fn try_new(
                id: EntityId,
                entity: &'m Entity,
                release: SchemaVersion,
            ) -> ApprovalResult<Self> {
                Self::bound(id, entity, Layout::of_version(release)?)
            }
            pub(crate) fn bound(
                id: EntityId,
                entity: &'m Entity,
                layout: Layout,
            ) -> ApprovalResult<Self> {
                if !entity.is_type($kind) {
                    return Err(wrong($kind, entity));
                }
                layout.require_entity($kind)?;
                Ok(Self {
                    record: Record {
                        kind: $kind,
                        id,
                        entity,
                        layout,
                    },
                })
            }
            /// Stable model identifier.
            #[must_use]
            pub const fn id(self) -> EntityId {
                self.record.id
            }
            /// The release this projection reads against.
            #[must_use]
            pub const fn release(self) -> SchemaVersion {
                self.record.layout.version()
            }
        }
    };
}

projection!(Approval, APPROVAL);
projection!(ApprovalRelationship, APPROVAL_REL);
projection!(ResourceApprovalRelationship, RESOURCE_REL);
projection!(ApprovalAssignment, ASSIGNMENT);

/// Fail with `MissingAttribute` when the release requires `attribute` and
/// the record leaves it `$`.
fn required_if_declared(record: Record<'_>, attribute: &'static str) -> ApprovalResult<()> {
    if record.requires(attribute) {
        record.required_text(attribute)?;
    }
    Ok(())
}

impl<'m> Approval<'m> {
    /// Optional approval identifier (required in IFC2X3).
    pub fn identifier(self) -> ApprovalResult<Option<&'m str>> {
        self.record.optional_text("Identifier")
    }
    /// Optional approval name (required in IFC2X3).
    pub fn name(self) -> ApprovalResult<Option<&'m str>> {
        self.record.optional_text("Name")
    }
    /// Optional description.
    pub fn description(self) -> ApprovalResult<Option<&'m str>> {
        self.record.optional_text("Description")
    }
    /// Optional authored approval time, IFC4/IFC4X3 `IfcDateTime` text.
    ///
    /// IFC2X3's `ApprovalDateTime` (which IFC4 renamed to it) is an
    /// `IfcDateTimeSelect` record: that is `StructuredValue` with the
    /// record's id, never read as text.
    pub fn time_of_approval(self) -> ApprovalResult<Option<&'m str>> {
        self.record.optional_text("TimeOfApproval")
    }
    /// Optional status label. `NotInSchema` in IFC2X3.
    pub fn status(self) -> ApprovalResult<Option<&'m str>> {
        self.record.optional_text("Status")
    }
    /// Optional approval level label. `NotInSchema` in IFC2X3.
    pub fn level(self) -> ApprovalResult<Option<&'m str>> {
        self.record.optional_text("Level")
    }
    /// Optional qualifier text. `NotInSchema` in IFC2X3.
    pub fn qualifier(self) -> ApprovalResult<Option<&'m str>> {
        self.record.optional_text("Qualifier")
    }
    /// Optional requesting actor-select reference. `NotInSchema` in IFC2X3,
    /// which relates actors through `IfcApprovalActorRelationship`.
    pub fn requesting_approval(self) -> ApprovalResult<Option<EntityId>> {
        self.record.optional_ref("RequestingApproval")
    }
    /// Optional giving actor-select reference. `NotInSchema` in IFC2X3.
    pub fn giving_approval(self) -> ApprovalResult<Option<EntityId>> {
        self.record.optional_ref("GivingApproval")
    }

    fn validate(self, view: ApprovalView<'m>) -> ApprovalResult<Self> {
        let record = self.record;
        required_if_declared(record, "Identifier")?;
        required_if_declared(record, "Name")?;
        if self.identifier()?.is_none() && self.name()?.is_none() {
            return Err(ApprovalError::Semantic {
                entity: APPROVAL,
                id: record.id,
                rule: "HasIdentifierOrName",
                detail: "Identifier and Name are both absent".into(),
            });
        }
        match self.time_of_approval() {
            Ok(_) => {}
            Err(ApprovalError::StructuredValue { target, .. }) => record.validate_target(
                view.model(),
                "TimeOfApproval",
                target,
                "IfcDateTimeSelect",
            )?,
            Err(error) => return Err(error),
        }
        // IFC2X3 requires ApprovalDateTime; a `$` is a missing value.
        if record.requires("TimeOfApproval") && matches!(self.time_of_approval(), Ok(None)) {
            return Err(ApprovalError::MissingAttribute {
                entity: APPROVAL,
                id: record.id,
                attribute: "TimeOfApproval",
            });
        }
        for attribute in ["RequestingApproval", "GivingApproval"] {
            if !record.declares(attribute) {
                continue;
            }
            if let Some(target) = record.optional_ref(attribute)? {
                record.validate_target(view.model(), attribute, target, "IfcActorSelect")?;
            }
        }
        Ok(self)
    }
}

impl<'m> ApprovalRelationship<'m> {
    /// Optional relationship name (required in IFC2X3).
    pub fn name(self) -> ApprovalResult<Option<&'m str>> {
        self.record.optional_text("Name")
    }
    /// Optional relationship description.
    pub fn description(self) -> ApprovalResult<Option<&'m str>> {
        self.record.optional_text("Description")
    }
    /// Relating approval.
    pub fn relating_approval(self) -> ApprovalResult<EntityId> {
        self.record.required_ref("RelatingApproval")
    }
    /// Non-empty unique related approvals; IFC2X3's single
    /// `RelatedApproval` is a one-element set.
    pub fn related_approvals(self) -> ApprovalResult<Vec<EntityId>> {
        self.record.required_refs("RelatedApprovals")
    }
    fn validate(self, view: ApprovalView<'m>) -> ApprovalResult<Self> {
        let record = self.record;
        required_if_declared(record, "Name")?;
        let relating = self.relating_approval()?;
        record.validate_target(view.model(), "RelatingApproval", relating, APPROVAL)?;
        for target in self.related_approvals()? {
            if target == relating {
                return Err(ApprovalError::Semantic {
                    entity: APPROVAL_REL,
                    id: record.id,
                    rule: "NoSelfRelationship",
                    detail: format!("approval {target} appears on both ends"),
                });
            }
            record.validate_target(view.model(), "RelatedApprovals", target, APPROVAL)?;
        }
        Ok(self)
    }
}

impl<'m> ResourceApprovalRelationship<'m> {
    /// Optional relationship name.
    pub fn name(self) -> ApprovalResult<Option<&'m str>> {
        self.record.optional_text("Name")
    }
    /// Optional relationship description.
    pub fn description(self) -> ApprovalResult<Option<&'m str>> {
        self.record.optional_text("Description")
    }
    /// Non-empty unique resource-select targets.
    pub fn related_resources(self) -> ApprovalResult<Vec<EntityId>> {
        self.record.required_refs("RelatedResourceObjects")
    }
    /// Approval governing the resources.
    pub fn relating_approval(self) -> ApprovalResult<EntityId> {
        self.record.required_ref("RelatingApproval")
    }
    fn validate(self, view: ApprovalView<'m>) -> ApprovalResult<Self> {
        let record = self.record;
        record.validate_target(
            view.model(),
            "RelatingApproval",
            self.relating_approval()?,
            APPROVAL,
        )?;
        for target in self.related_resources()? {
            record.validate_target(
                view.model(),
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
        self.record.required_text("GlobalId")
    }
    /// Optional relationship name.
    pub fn name(self) -> ApprovalResult<Option<&'m str>> {
        self.record.optional_text("Name")
    }
    /// Optional relationship description.
    pub fn description(self) -> ApprovalResult<Option<&'m str>> {
        self.record.optional_text("Description")
    }
    /// Related definition-select targets (`IfcRoot` under WR21 in IFC2X3).
    pub fn related_objects(self) -> ApprovalResult<Vec<EntityId>> {
        self.record.required_refs("RelatedObjects")
    }
    /// Relating approval.
    pub fn relating_approval(self) -> ApprovalResult<EntityId> {
        self.record.required_ref("RelatingApproval")
    }
    fn validate(self, view: ApprovalView<'m>) -> ApprovalResult<Self> {
        let record = self.record;
        if Guid::parse(self.global_id()?).is_none() {
            return Err(ApprovalError::InvalidValue {
                entity: ASSIGNMENT,
                id: record.id,
                attribute: "GlobalId",
                value: self.global_id()?.into(),
            });
        }
        record.validate_target(
            view.model(),
            "RelatingApproval",
            self.relating_approval()?,
            APPROVAL,
        )?;
        for target in self.related_objects()? {
            record.validate_target(
                view.model(),
                "RelatedObjects",
                target,
                "IfcDefinitionSelect",
            )?;
        }
        Ok(self)
    }
}

impl<'m> ApprovalView<'m> {
    fn entity(self, id: EntityId) -> ApprovalResult<&'m Entity> {
        self.model()
            .get(id)
            .ok_or(ApprovalError::UnknownEntity { id })
    }

    /// Strictly project one approval.
    pub fn approval(self, id: EntityId) -> ApprovalResult<Approval<'m>> {
        Approval::bound(id, self.entity(id)?, self.layout()?)?.validate(self)
    }
    /// Strictly project one approval-to-approval relationship.
    pub fn approval_relationship(self, id: EntityId) -> ApprovalResult<ApprovalRelationship<'m>> {
        ApprovalRelationship::bound(id, self.entity(id)?, self.layout()?)?.validate(self)
    }
    /// Strictly project one resource approval relationship.
    pub fn resource_approval_relationship(
        self,
        id: EntityId,
    ) -> ApprovalResult<ResourceApprovalRelationship<'m>> {
        ResourceApprovalRelationship::bound(id, self.entity(id)?, self.layout()?)?.validate(self)
    }
    /// Strictly project one rooted object approval association.
    pub fn approval_assignment(self, id: EntityId) -> ApprovalResult<ApprovalAssignment<'m>> {
        ApprovalAssignment::bound(id, self.entity(id)?, self.layout()?)?.validate(self)
    }

    /// Resource-select IDs directly governed by an approval.
    ///
    /// Empty in IFC2X3, which declares no `IfcResourceApprovalRelationship`.
    pub fn resources_approved_by(self, approval: EntityId) -> ApprovalResult<Vec<EntityId>> {
        self.approval(approval)?;
        let layout = self.layout()?;
        let mut out = Vec::new();
        if layout.require_entity(RESOURCE_REL).is_err() {
            return Ok(out);
        }
        for (id, entity) in self.model().of_type(RESOURCE_REL) {
            let relationship = ResourceApprovalRelationship::bound(id, entity, layout)?;
            if relationship.relating_approval()? == approval {
                out.extend(relationship.validate(self)?.related_resources()?);
            }
        }
        Ok(out)
    }

    /// Definition-select IDs directly associated with an approval.
    pub fn objects_approved_by(self, approval: EntityId) -> ApprovalResult<Vec<EntityId>> {
        self.approval(approval)?;
        let layout = self.layout()?;
        let mut out = Vec::new();
        for (id, entity) in self.model().of_type(ASSIGNMENT) {
            let assignment = ApprovalAssignment::bound(id, entity, layout)?;
            if assignment.relating_approval()? == approval {
                out.extend(assignment.validate(self)?.related_objects()?);
            }
        }
        Ok(out)
    }
}
