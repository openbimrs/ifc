//! `IfcRelAssociatesConstraint`, the one rooted record of this crate.
//!
//! It is the one writer bound to the model's declared release (see
//! `release.rs`), because it carries `IfcRoot.OwnerHistory`, which IFC2X3
//! requires.

use std::collections::HashSet;

use ifc_model::guid::Guid;
use ifc_model::{EntityId, Model, Transaction, Value};

use crate::authoring::{final_type, optional_ref, optional_text, refs, text, validate_target_in};
use crate::release::{bind, require_owner_history};
use crate::{ConstraintError, ConstraintResult};

const ASSIGNMENT: &str = "IFCRELASSOCIATESCONSTRAINT";

/// Draft for one rooted constraint association.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct ConstraintAssociationDraft<'a> {
    /// Compressed IFC GlobalId.
    pub global_id: &'a str,
    /// Optional relationship name.
    pub name: Option<&'a str>,
    /// Optional relationship description.
    pub description: Option<&'a str>,
    /// Non-empty unique definition-select targets.
    pub related_objects: &'a [EntityId],
    /// Optional association intent.
    pub intent: Option<&'a str>,
    /// Existing or earlier-staged metric/objective.
    pub relating_constraint: EntityId,
}

impl<'a> ConstraintAssociationDraft<'a> {
    /// Starts a draft from its required fields; every other field is unset.
    #[must_use]
    pub fn new(
        global_id: &'a str,
        related_objects: &'a [EntityId],
        relating_constraint: EntityId,
    ) -> Self {
        Self {
            global_id,
            name: None,
            description: None,
            related_objects,
            intent: None,
            relating_constraint,
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

    /// Sets `intent`: Optional association intent.
    #[must_use]
    pub fn intent(mut self, value: &'a str) -> Self {
        self.intent = Some(value);
        self
    }
}

/// Validate and stage one rooted constraint association.
///
/// # Release
///
/// Bound to the model's declared release (#202): the record is laid out by
/// attribute name from that release's table, and its references are checked
/// against it (`RelatedObjects` is `IfcDefinitionSelect` in IFC4 and IFC4X3,
/// `IfcRoot` restricted by WR21 in IFC2X3). `OwnerHistory` is left `$`,
/// which IFC4 and IFC4X3 allow and IFC2X3 does not, so an IFC2X3 model is
/// refused with [`ConstraintError::AuthoringRequired`]. Use
/// [`associate_constraint_with_owner_history`] there. IFC2X3 also requires
/// `Intent`, which IFC4 made optional.
///
/// # Errors
///
/// A malformed GlobalId, a relating constraint that is not an
/// `IfcConstraint`, an empty, duplicated or out-of-type `RelatedObjects`; a
/// model that binds no single known release
/// ([`ConstraintError::MultipleSchemas`],
/// [`ConstraintError::UnsupportedSchema`]), and an IFC2X3 model. Nothing is
/// staged on an error.
pub fn associate_constraint(
    tx: &mut Transaction,
    model: &Model,
    draft: ConstraintAssociationDraft<'_>,
) -> ConstraintResult<EntityId> {
    associate(tx, model, draft, None)
}

/// [`associate_constraint`] with a caller-supplied `IfcOwnerHistory`, which
/// IFC2X3 requires on every `IfcRoot`.
///
/// `owner_history` must be in the model or staged earlier on `tx`, and must
/// be an `IfcOwnerHistory`; one is never invented here (build it with
/// `ifc-author`). In IFC4 and IFC4X3 the reference fills the optional slot.
///
/// # Errors
///
/// Those of [`associate_constraint`] except the IFC2X3 `OwnerHistory`
/// refusal (an IFC2X3 association without `intent` is still refused with
/// [`ConstraintError::AuthoringRequired`]), and
/// [`ConstraintError::UnknownEntity`] for an `owner_history` that does not
/// resolve or [`ConstraintError::AuthoringReferenceType`] for one that is
/// not an `IfcOwnerHistory`. Nothing is staged on an error.
pub fn associate_constraint_with_owner_history(
    tx: &mut Transaction,
    model: &Model,
    draft: ConstraintAssociationDraft<'_>,
    owner_history: EntityId,
) -> ConstraintResult<EntityId> {
    associate(tx, model, draft, Some(owner_history))
}

/// Stage an `IfcRelAssociatesConstraint`; `None` leaves `OwnerHistory` `$`.
fn associate(
    tx: &mut Transaction,
    model: &Model,
    draft: ConstraintAssociationDraft<'_>,
    owner_history: Option<EntityId>,
) -> ConstraintResult<EntityId> {
    let layout = bind(model)?;
    if Guid::parse(draft.global_id).is_none() {
        return Err(ConstraintError::AuthoringInvalid {
            entity: ASSIGNMENT,
            attribute: "GlobalId",
            value: draft.global_id.into(),
        });
    }
    validate_target_in(
        layout.schema(),
        tx,
        model,
        draft.relating_constraint,
        "IfcConstraint",
    )?;
    if draft.related_objects.is_empty() {
        return Err(ConstraintError::AuthoringInvalid {
            entity: ASSIGNMENT,
            attribute: "RelatedObjects",
            value: "empty SET [1:?]".into(),
        });
    }
    let mut seen = HashSet::new();
    for &target in draft.related_objects {
        if !seen.insert(target) {
            return Err(ConstraintError::AuthoringInvalid {
                entity: ASSIGNMENT,
                attribute: "RelatedObjects",
                value: format!("duplicate {target}"),
            });
        }
        let actual =
            final_type(tx, model, target).ok_or(ConstraintError::UnknownEntity { id: target })?;
        if !layout.is_definition(actual) {
            return Err(ConstraintError::AuthoringReferenceType {
                target,
                expected: "IfcDefinitionSelect",
                actual: actual.into(),
            });
        }
    }
    if let Some(owner_history) = owner_history {
        require_owner_history(tx, model, owner_history)?;
    }
    let record = layout.named_record(
        ASSIGNMENT,
        vec![
            ("GlobalId", text(draft.global_id)),
            ("OwnerHistory", optional_ref(owner_history)),
            ("Name", optional_text(draft.name)),
            ("Description", optional_text(draft.description)),
            ("RelatedObjects", refs(draft.related_objects)),
            ("Intent", optional_text(draft.intent)),
            ("RelatingConstraint", Value::Ref(draft.relating_constraint)),
        ],
    )?;
    Ok(tx.create(record))
}
