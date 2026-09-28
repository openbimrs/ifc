//! Generic `IfcExternalReferenceRelationship` projection and authoring.

use std::collections::HashSet;

use ifc_model::{EntityId, Model, Transaction, Value};

use crate::authoring::{require_accepts, text};
use crate::release::Release;
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
        optional_text(
            KIND,
            self.id(),
            self.entity(),
            self.text_slot("Name")?,
            "Name",
        )
    }

    /// Optional relationship description.
    pub fn description(self) -> ClassificationResult<Option<&'m str>> {
        optional_text(
            KIND,
            self.id(),
            self.entity(),
            self.text_slot("Description")?,
            "Description",
        )
    }

    /// External reference that applies to the related resources.
    pub fn relating_reference(self) -> ClassificationResult<EntityId> {
        required_ref(
            KIND,
            self.id(),
            self.entity(),
            self.slot("RelatingReference")?,
            "RelatingReference",
        )
    }

    /// Resource-level objects carrying this external reference.
    pub fn related_resources(self) -> ClassificationResult<Vec<EntityId>> {
        required_refs(
            KIND,
            self.id(),
            self.entity(),
            self.slot("RelatedResourceObjects")?,
            "RelatedResourceObjects",
        )
    }

    fn validate(self, model: &Model) -> ClassificationResult<Self> {
        validate_target(self, model, "RelatingReference", self.relating_reference()?)?;
        for target in self.related_resources()? {
            validate_target(self, model, "RelatedResourceObjects", target)?;
        }
        Ok(self)
    }
}

/// Check `target` against the type the bound release declares for
/// `attribute`: `IfcExternalReference` and `IfcResourceObjectSelect`, whose
/// members differ by release (IFC4X3 adds `IfcShapeAspect`). IFC2X3 has no
/// `IfcExternalReferenceRelationship`, so any read there already fails
/// `NotInSchema` before this is reached.
fn validate_target(
    relationship: ExternalReferenceRelationship<'_>,
    model: &Model,
    attribute: &'static str,
    target: EntityId,
) -> ClassificationResult<()> {
    let relation = relationship.id();
    let entity = model
        .get(target)
        .ok_or(ClassificationError::DanglingReference {
            entity: KIND,
            id: relation,
            attribute,
            target,
        })?;
    let release = relationship.release();
    if release.accepts(KIND, relation, attribute, &entity.type_name)? {
        Ok(())
    } else {
        Err(ClassificationError::ReferenceType {
            entity: KIND,
            id: relation,
            attribute,
            target,
            expected: release.declared_type(KIND, relation, attribute)?,
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
        ExternalReferenceRelationship::try_bound(id, entity, self.release())?.validate(self.model())
    }

    /// All generic external-reference relationships in deterministic model order.
    pub fn external_reference_relationships(
        self,
    ) -> impl Iterator<Item = ExternalReferenceRelationship<'m>> + 'm {
        self.model().of_type(KIND).map(move |(id, entity)| {
            ExternalReferenceRelationship::from_known(id, entity, self.release())
        })
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

/// Draft for one generic external-reference relationship (IFC4 onwards).
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct ExternalReferenceRelationshipDraft<'a> {
    /// Optional relationship name.
    pub name: Option<&'a str>,
    /// Optional relationship description.
    pub description: Option<&'a str>,
    /// Existing or earlier-staged subtype of `IfcExternalReference`.
    pub relating_reference: EntityId,
    /// Non-empty unique `IfcResourceObjectSelect` targets, as the model's
    /// release declares that select (IFC4X3 adds `IfcShapeAspect`).
    pub related_resources: &'a [EntityId],
}

impl<'a> ExternalReferenceRelationshipDraft<'a> {
    /// Starts a draft from its required fields; every other field is unset.
    #[must_use]
    pub fn new(relating_reference: EntityId, related_resources: &'a [EntityId]) -> Self {
        Self {
            name: None,
            description: None,
            relating_reference,
            related_resources,
        }
    }

    /// Sets `name`: Optional relationship name.
    #[must_use]
    pub fn name(mut self, value: &'a str) -> Self {
        self.name = Some(value);
        self
    }

    /// Sets `description`: Optional relationship description.
    #[must_use]
    pub fn description(mut self, value: &'a str) -> Self {
        self.description = Some(value);
        self
    }
}

/// Validate and stage one `IfcExternalReferenceRelationship` in the layout
/// of `model`'s declared release.
///
/// # Errors
///
/// An empty or duplicated related set; a reference the release's declared
/// type does not accept; an IFC2X3 model, which has no such entity
/// (`EntityNotInSchema`); and a header binding no single known release.
/// Nothing is staged on an error.
pub fn create_external_reference_relationship(
    tx: &mut Transaction,
    model: &Model,
    draft: ExternalReferenceRelationshipDraft<'_>,
) -> ClassificationResult<EntityId> {
    let release = Release::of(model);
    release.require_entity(KIND)?;
    if draft.related_resources.is_empty() {
        return Err(ClassificationError::AuthoringInvalid {
            entity: KIND,
            attribute: "RelatedResourceObjects",
            value: "empty SET [1:?]".into(),
        });
    }
    require_accepts(
        tx,
        model,
        release,
        KIND,
        "RelatingReference",
        draft.relating_reference,
    )?;
    let mut seen = HashSet::new();
    for &target in draft.related_resources {
        if !seen.insert(target) {
            return Err(ClassificationError::AuthoringInvalid {
                entity: KIND,
                attribute: "RelatedResourceObjects",
                value: format!("duplicate {target}"),
            });
        }
        require_accepts(tx, model, release, KIND, "RelatedResourceObjects", target)?;
    }
    let record = release.record(
        KIND,
        vec![
            ("Name", draft.name.map_or(Value::Null, text)),
            ("Description", draft.description.map_or(Value::Null, text)),
            ("RelatingReference", Value::Ref(draft.relating_reference)),
            (
                "RelatedResourceObjects",
                Value::List(
                    draft
                        .related_resources
                        .iter()
                        .copied()
                        .map(Value::Ref)
                        .collect(),
                ),
            ),
        ],
    )?;
    Ok(tx.create(record))
}
