//! Generic `IfcExternalReferenceRelationship` projection and authoring.

use std::collections::HashSet;

use ifc_model::{Entity, EntityId, Model, Transaction, Value};

use crate::authoring::{final_type, text};
use crate::view::{
    borrowed_entity, optional_text, required_ref, required_refs, ClassificationView,
};
use crate::{ClassificationError, ClassificationResult};

const KIND: &str = "IFCEXTERNALREFERENCERELATIONSHIP";

borrowed_entity!(
    ExternalReferenceRelationship,
    "IFCEXTERNALREFERENCERELATIONSHIP"
);

impl<'m> ExternalReferenceRelationship<'m> {
    /// Optional relationship name.
    pub fn name(self) -> ClassificationResult<Option<&'m str>> {
        optional_text(KIND, self.id(), self.entity(), 0, "Name")
    }

    /// Optional relationship description.
    pub fn description(self) -> ClassificationResult<Option<&'m str>> {
        optional_text(KIND, self.id(), self.entity(), 1, "Description")
    }

    /// External reference that applies to the related resources.
    pub fn relating_reference(self) -> ClassificationResult<EntityId> {
        required_ref(KIND, self.id(), self.entity(), 2, "RelatingReference")
    }

    /// Resource-level objects carrying this external reference.
    pub fn related_resources(self) -> ClassificationResult<Vec<EntityId>> {
        required_refs(KIND, self.id(), self.entity(), 3, "RelatedResourceObjects")
    }

    fn validate(self, model: &Model) -> ClassificationResult<Self> {
        validate_target(
            model,
            self.id(),
            "RelatingReference",
            self.relating_reference()?,
            "IfcExternalReference",
        )?;
        for target in self.related_resources()? {
            validate_target(
                model,
                self.id(),
                "RelatedResourceObjects",
                target,
                "IfcResourceObjectSelect",
            )?;
        }
        Ok(self)
    }
}

fn validate_target(
    model: &Model,
    relation: EntityId,
    attribute: &'static str,
    target: EntityId,
    expected: &'static str,
) -> ClassificationResult<()> {
    let entity = model
        .get(target)
        .ok_or(ClassificationError::DanglingReference {
            entity: KIND,
            id: relation,
            attribute,
            target,
        })?;
    if ifc_schema::ifc4().accepts_type(expected, &entity.type_name) {
        Ok(())
    } else {
        Err(ClassificationError::ReferenceType {
            entity: KIND,
            id: relation,
            attribute,
            target,
            expected,
            actual: entity.type_name.to_string(),
        })
    }
}

impl<'m> ClassificationView<'m> {
    /// Strictly project one generic external-reference relationship.
    pub fn external_reference_relationship(
        self,
        id: EntityId,
    ) -> ClassificationResult<ExternalReferenceRelationship<'m>> {
        let entity = self
            .model()
            .get(id)
            .ok_or(ClassificationError::UnknownEntity { id })?;
        ExternalReferenceRelationship::try_new(id, entity)?.validate(self.model())
    }

    /// All generic external-reference relationships in deterministic model order.
    pub fn external_reference_relationships(
        self,
    ) -> impl Iterator<Item = ExternalReferenceRelationship<'m>> + 'm {
        self.model()
            .of_type(KIND)
            .map(|(id, entity)| ExternalReferenceRelationship::from_known(id, entity))
    }

    /// Valid external-reference relationships naming a particular resource.
    pub fn external_references_for(
        self,
        resource: EntityId,
    ) -> ClassificationResult<Vec<ExternalReferenceRelationship<'m>>> {
        if self.model().get(resource).is_none() {
            return Err(ClassificationError::UnknownEntity { id: resource });
        }
        let mut out = Vec::new();
        for relationship in self.external_reference_relationships() {
            if relationship.related_resources()?.contains(&resource) {
                out.push(relationship.validate(self.model())?);
            }
        }
        Ok(out)
    }
}

/// Draft for one IFC4 generic external-reference relationship.
#[derive(Debug, Clone, Copy)]
pub struct ExternalReferenceRelationshipDraft<'a> {
    /// Optional relationship name.
    pub name: Option<&'a str>,
    /// Optional relationship description.
    pub description: Option<&'a str>,
    /// Existing or earlier-staged subtype of `IfcExternalReference`.
    pub relating_reference: EntityId,
    /// Non-empty unique `IfcResourceObjectSelect` targets.
    pub related_resources: &'a [EntityId],
}

/// Validate and stage one `IfcExternalReferenceRelationship`.
pub fn create_external_reference_relationship(
    tx: &mut Transaction,
    model: &Model,
    draft: ExternalReferenceRelationshipDraft<'_>,
) -> ClassificationResult<EntityId> {
    if draft.related_resources.is_empty() {
        return Err(ClassificationError::AuthoringInvalid {
            entity: KIND,
            attribute: "RelatedResourceObjects",
            value: "empty SET [1:?]".into(),
        });
    }
    validate_draft_target(tx, model, draft.relating_reference, "IfcExternalReference")?;
    let mut seen = HashSet::new();
    for &target in draft.related_resources {
        if !seen.insert(target) {
            return Err(ClassificationError::AuthoringInvalid {
                entity: KIND,
                attribute: "RelatedResourceObjects",
                value: format!("duplicate {target}"),
            });
        }
        validate_draft_target(tx, model, target, "IfcResourceObjectSelect")?;
    }
    Ok(tx.create(Entity::new(
        KIND,
        vec![
            draft.name.map_or(Value::Null, text),
            draft.description.map_or(Value::Null, text),
            Value::Ref(draft.relating_reference),
            Value::List(
                draft
                    .related_resources
                    .iter()
                    .copied()
                    .map(Value::Ref)
                    .collect(),
            ),
        ],
    )))
}

fn validate_draft_target(
    tx: &Transaction,
    model: &Model,
    target: EntityId,
    expected: &'static str,
) -> ClassificationResult<()> {
    let actual =
        final_type(tx, model, target).ok_or(ClassificationError::UnknownEntity { id: target })?;
    if ifc_schema::ifc4().accepts_type(expected, actual) {
        Ok(())
    } else {
        Err(ClassificationError::AuthoringReferenceType {
            target,
            expected,
            actual: actual.into(),
        })
    }
}
