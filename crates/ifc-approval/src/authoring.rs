//! Transaction-staged authoring for the bounded approval domain.
//!
//! `IfcApproval` and the two approval relationships are resources, not
//! `IfcRoot` subtypes, so they carry no `GlobalId`; never invent one for
//! them. Only `IfcRelAssociatesApproval` is rooted, and its caller-supplied
//! GlobalId is validated before staging; it carries `IfcRoot.OwnerHistory`,
//! which IFC2X3 requires.
//!
//! Every writer binds the model's declared release and lays its record out
//! by attribute name from that release's table (see `release.rs`, #202,
//! #212). What the release cannot hold is refused, never dropped or
//! written into a slot that means something else.

use std::collections::HashSet;
use std::sync::Arc;

use ifc_model::guid::Guid;
use ifc_model::{Edit, EntityId, Model, Transaction, Value};

use ifc_schema::Schema;

use crate::release::{bind, require_owner_history};
use crate::{ApprovalError, ApprovalResult};

const APPROVAL: &str = "IFCAPPROVAL";
const APPROVAL_REL: &str = "IFCAPPROVALRELATIONSHIP";
const RESOURCE_REL: &str = "IFCRESOURCEAPPROVALRELATIONSHIP";
const ASSIGNMENT: &str = "IFCRELASSOCIATESAPPROVAL";

/// A date and time for an authoring draft, in the form the release
/// declares it.
///
/// IFC4 and IFC4X3 declare `IfcDateTime`, ISO 8601 text written as given;
/// IFC2X3 declares `IfcDateTimeSelect`, a reference to an existing or
/// earlier-staged `IfcCalendarDate`, `IfcLocalTime` or `IfcDateAndTime`.
/// A form the release does not declare is refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum DateTimeInput<'a> {
    /// IFC4/IFC4X3 `IfcDateTime` text.
    Text(&'a str),
    /// An IFC2X3 `IfcDateTimeSelect` record.
    Record(EntityId),
}

impl<'a> From<&'a str> for DateTimeInput<'a> {
    fn from(text: &'a str) -> Self {
        Self::Text(text)
    }
}

impl From<EntityId> for DateTimeInput<'_> {
    fn from(record: EntityId) -> Self {
        Self::Record(record)
    }
}

/// Draft for one `IfcApproval`.
#[derive(Debug, Clone, Copy, Default)]
#[non_exhaustive]
pub struct ApprovalDraft<'a> {
    /// Optional identifier; at least this or `name` is required.
    pub identifier: Option<&'a str>,
    /// Optional name; at least this or `identifier` is required.
    pub name: Option<&'a str>,
    /// Optional description.
    pub description: Option<&'a str>,
    /// Optional approval time: IFC4/IFC4X3 `IfcDateTime` text, or an
    /// existing or earlier-staged IFC2X3 `IfcDateTimeSelect` record
    /// (`ApprovalDateTime`, which IFC2X3 requires).
    pub time_of_approval: Option<DateTimeInput<'a>>,
    /// Optional status label.
    pub status: Option<&'a str>,
    /// Optional level label.
    pub level: Option<&'a str>,
    /// Optional qualifier text.
    pub qualifier: Option<&'a str>,
    /// Optional existing or earlier-staged `IfcActorSelect` target.
    pub requesting_approval: Option<EntityId>,
    /// Optional existing or earlier-staged `IfcActorSelect` target.
    pub giving_approval: Option<EntityId>,
}

impl<'a> ApprovalDraft<'a> {
    /// Starts a draft with every field unset.
    #[must_use]
    pub fn new() -> Self {
        Self {
            identifier: None,
            name: None,
            description: None,
            time_of_approval: None,
            status: None,
            level: None,
            qualifier: None,
            requesting_approval: None,
            giving_approval: None,
        }
    }

    /// Sets [`Self::identifier`]: Optional identifier; at least this or `name` is required.
    #[must_use]
    pub fn identifier(mut self, value: &'a str) -> Self {
        self.identifier = Some(value);
        self
    }

    /// Sets [`Self::name`]: Optional name; at least this or `identifier` is required.
    #[must_use]
    pub fn name(mut self, value: &'a str) -> Self {
        self.name = Some(value);
        self
    }

    /// Sets [`Self::description`]: Optional description.
    #[must_use]
    pub fn description(mut self, value: &'a str) -> Self {
        self.description = Some(value);
        self
    }

    /// Sets [`Self::time_of_approval`]: text, or an IFC2X3 date record.
    #[must_use]
    pub fn time_of_approval(mut self, value: impl Into<DateTimeInput<'a>>) -> Self {
        self.time_of_approval = Some(value.into());
        self
    }

    /// Sets [`Self::status`]: Optional status label.
    #[must_use]
    pub fn status(mut self, value: &'a str) -> Self {
        self.status = Some(value);
        self
    }

    /// Sets [`Self::level`]: Optional level label.
    #[must_use]
    pub fn level(mut self, value: &'a str) -> Self {
        self.level = Some(value);
        self
    }

    /// Sets [`Self::qualifier`]: Optional qualifier text.
    #[must_use]
    pub fn qualifier(mut self, value: &'a str) -> Self {
        self.qualifier = Some(value);
        self
    }

    /// Sets [`Self::requesting_approval`]: Optional existing or earlier-staged `IfcActorSelect` target.
    #[must_use]
    pub fn requesting_approval(mut self, value: EntityId) -> Self {
        self.requesting_approval = Some(value);
        self
    }

    /// Sets [`Self::giving_approval`]: Optional existing or earlier-staged `IfcActorSelect` target.
    #[must_use]
    pub fn giving_approval(mut self, value: EntityId) -> Self {
        self.giving_approval = Some(value);
        self
    }
}

/// Draft for one direct approval relationship.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct ApprovalRelationshipDraft<'a> {
    /// Optional relationship name.
    pub name: Option<&'a str>,
    /// Optional relationship description.
    pub description: Option<&'a str>,
    /// Existing or earlier-staged approval.
    pub relating_approval: EntityId,
    /// Non-empty unique related approvals.
    pub related_approvals: &'a [EntityId],
}

impl<'a> ApprovalRelationshipDraft<'a> {
    /// Starts a draft with its required fields; the rest are unset.
    #[must_use]
    pub fn new(relating_approval: EntityId, related_approvals: &'a [EntityId]) -> Self {
        Self {
            name: None,
            description: None,
            relating_approval,
            related_approvals,
        }
    }

    /// Sets [`Self::name`]: Optional relationship name.
    #[must_use]
    pub fn name(mut self, value: &'a str) -> Self {
        self.name = Some(value);
        self
    }

    /// Sets [`Self::description`]: Optional relationship description.
    #[must_use]
    pub fn description(mut self, value: &'a str) -> Self {
        self.description = Some(value);
        self
    }
}

/// Draft for one approval-to-resource relationship.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct ResourceApprovalDraft<'a> {
    /// Optional relationship name.
    pub name: Option<&'a str>,
    /// Optional relationship description.
    pub description: Option<&'a str>,
    /// Non-empty unique `IfcResourceObjectSelect` targets.
    pub related_resources: &'a [EntityId],
    /// Existing or earlier-staged approval.
    pub relating_approval: EntityId,
}

impl<'a> ResourceApprovalDraft<'a> {
    /// Starts a draft with its required fields; the rest are unset.
    #[must_use]
    pub fn new(related_resources: &'a [EntityId], relating_approval: EntityId) -> Self {
        Self {
            name: None,
            description: None,
            related_resources,
            relating_approval,
        }
    }

    /// Sets [`Self::name`]: Optional relationship name.
    #[must_use]
    pub fn name(mut self, value: &'a str) -> Self {
        self.name = Some(value);
        self
    }

    /// Sets [`Self::description`]: Optional relationship description.
    #[must_use]
    pub fn description(mut self, value: &'a str) -> Self {
        self.description = Some(value);
        self
    }
}

/// Draft for one rooted approval association to definitions.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct ApprovalAssociationDraft<'a> {
    /// Compressed IFC GlobalId.
    pub global_id: &'a str,
    /// Optional relationship name.
    pub name: Option<&'a str>,
    /// Optional relationship description.
    pub description: Option<&'a str>,
    /// Non-empty unique `IfcDefinitionSelect` targets.
    pub related_objects: &'a [EntityId],
    /// Existing or earlier-staged approval.
    pub relating_approval: EntityId,
}

impl<'a> ApprovalAssociationDraft<'a> {
    /// Starts a draft with its required fields; the rest are unset.
    #[must_use]
    pub fn new(
        global_id: &'a str,
        related_objects: &'a [EntityId],
        relating_approval: EntityId,
    ) -> Self {
        Self {
            global_id,
            name: None,
            description: None,
            related_objects,
            relating_approval,
        }
    }

    /// Sets [`Self::name`]: Optional relationship name.
    #[must_use]
    pub fn name(mut self, value: &'a str) -> Self {
        self.name = Some(value);
        self
    }

    /// Sets [`Self::description`]: Optional relationship description.
    #[must_use]
    pub fn description(mut self, value: &'a str) -> Self {
        self.description = Some(value);
        self
    }
}

/// Validate and stage one approval in the model's declared release.
///
/// # Release
///
/// IFC4 and IFC4X3 declare nine attributes and require at least one of
/// `Identifier` and `Name`. IFC2X3 declares seven and requires `Identifier`,
/// `Name` and `ApprovalDateTime` (the IFC4 `TimeOfApproval`) as an
/// `IfcDateTimeSelect` record; it declares no `Status`, `Level`,
/// `Qualifier`, `RequestingApproval` or `GivingApproval` under those names.
///
/// # Errors
///
/// Neither identifier nor name; an actor that is not an `IfcActorSelect`; a
/// date record that is not a date record of the release; a header binding
/// no single verified release (`MultipleSchemas`, `UnsupportedSchema`); a
/// value the release does not declare (`AuthoringNotInSchema`) or cannot
/// hold (`AuthoringValueType`: text where IFC2X3 declares a record, a
/// record where IFC4 declares text); and a required value left unset
/// (`AuthoringRequired`). Nothing is staged on an error.
pub fn create_approval(
    tx: &mut Transaction,
    model: &Model,
    draft: ApprovalDraft<'_>,
) -> ApprovalResult<EntityId> {
    let layout = bind(model)?;
    if draft.identifier.is_none() && draft.name.is_none() {
        return Err(ApprovalError::AuthoringInvalid {
            entity: APPROVAL,
            attribute: "WR",
            value: "Identifier and Name are both absent".into(),
        });
    }
    for target in [draft.requesting_approval, draft.giving_approval]
        .into_iter()
        .flatten()
    {
        validate_target_in(layout.schema(), tx, model, target, "IfcActorSelect")?;
    }
    let time = match draft.time_of_approval {
        None => Value::Null,
        Some(DateTimeInput::Text(text)) => Value::Text(Arc::from(text)),
        Some(DateTimeInput::Record(target)) => {
            if let Some((_, declared)) = layout.declared(APPROVAL, "TimeOfApproval") {
                if layout.admits_entity(&declared.type_name, 8) {
                    validate_target_in(
                        layout.schema(),
                        tx,
                        model,
                        target,
                        declared.type_name.as_str(),
                    )?;
                }
            }
            Value::Ref(target)
        }
    };
    let record = layout.named_record(
        APPROVAL,
        vec![
            ("Identifier", optional_text(draft.identifier)),
            ("Name", optional_text(draft.name)),
            ("Description", optional_text(draft.description)),
            ("TimeOfApproval", time),
            ("Status", optional_text(draft.status)),
            ("Level", optional_text(draft.level)),
            ("Qualifier", optional_text(draft.qualifier)),
            (
                "RequestingApproval",
                optional_ref(draft.requesting_approval),
            ),
            ("GivingApproval", optional_ref(draft.giving_approval)),
        ],
    )?;
    Ok(tx.create(record))
}

/// Validate and stage one approval-to-approval relationship in the model's
/// declared release.
///
/// IFC2X3 declares a single `RelatedApproval` and a required `Name`; IFC4
/// and IFC4X3 a set of `RelatedApprovals` and an optional `Name`.
///
/// # Errors
///
/// An empty, duplicated or self-referencing set; a target that is not an
/// `IfcApproval`; a header binding no single verified release; in IFC2X3
/// more than one related approval (`AuthoringValueType`) or no name
/// (`AuthoringRequired`). Nothing is staged on an error.
pub fn relate_approvals(
    tx: &mut Transaction,
    model: &Model,
    draft: ApprovalRelationshipDraft<'_>,
) -> ApprovalResult<EntityId> {
    let layout = bind(model)?;
    layout.require_entity(APPROVAL_REL)?;
    validate_target_in(
        layout.schema(),
        tx,
        model,
        draft.relating_approval,
        APPROVAL,
    )?;
    validate_set(
        layout.schema(),
        tx,
        model,
        APPROVAL_REL,
        "RelatedApprovals",
        draft.related_approvals,
        APPROVAL,
        Some(draft.relating_approval),
    )?;
    let single = layout
        .declared(APPROVAL_REL, "RelatedApprovals")
        .is_some_and(|(_, declared)| !declared.aggregate);
    let related = match (single, draft.related_approvals) {
        (false, related) => refs(related),
        (true, [one]) => Value::Ref(*one),
        (true, _) => {
            return Err(ApprovalError::AuthoringValueType {
                entity: APPROVAL_REL,
                attribute: "RelatedApprovals",
                declared: "IfcApproval",
                schema: layout.version(),
            })
        }
    };
    let record = layout.named_record(
        APPROVAL_REL,
        vec![
            ("Name", optional_text(draft.name)),
            ("Description", optional_text(draft.description)),
            ("RelatingApproval", Value::Ref(draft.relating_approval)),
            ("RelatedApprovals", related),
        ],
    )?;
    Ok(tx.create(record))
}

/// Validate and stage one approval-to-resource relationship in the model's
/// declared release.
///
/// # Errors
///
/// An empty or duplicated set; a relating target that is not an
/// `IfcApproval` or a resource outside `IfcResourceObjectSelect`; a header
/// binding no single verified release; and an IFC2X3 model, which declares
/// no `IfcResourceApprovalRelationship` (`EntityNotInSchema`). Nothing is
/// staged on an error.
pub fn relate_resource_approval(
    tx: &mut Transaction,
    model: &Model,
    draft: ResourceApprovalDraft<'_>,
) -> ApprovalResult<EntityId> {
    let layout = bind(model)?;
    layout.require_entity(RESOURCE_REL)?;
    validate_target_in(
        layout.schema(),
        tx,
        model,
        draft.relating_approval,
        APPROVAL,
    )?;
    validate_set(
        layout.schema(),
        tx,
        model,
        RESOURCE_REL,
        "RelatedResourceObjects",
        draft.related_resources,
        "IfcResourceObjectSelect",
        None,
    )?;
    let record = layout.named_record(
        RESOURCE_REL,
        vec![
            ("Name", optional_text(draft.name)),
            ("Description", optional_text(draft.description)),
            ("RelatedResourceObjects", refs(draft.related_resources)),
            ("RelatingApproval", Value::Ref(draft.relating_approval)),
        ],
    )?;
    Ok(tx.create(record))
}

/// Validate and stage one rooted approval association.
///
/// # Release
///
/// Bound to the model's declared release (#202): the record is laid out by
/// attribute name from that release's table, and its references are checked
/// against it (`RelatedObjects` is `IfcDefinitionSelect` in IFC4 and IFC4X3,
/// `IfcRoot` restricted by WR21 in IFC2X3). `OwnerHistory` is left `$`,
/// which IFC4 and IFC4X3 allow and IFC2X3 does not, so an IFC2X3 model is
/// refused with [`ApprovalError::AuthoringRequired`]. Use
/// [`associate_approval_with_owner_history`] there.
///
/// # Errors
///
/// A malformed GlobalId, a relating approval that is not an `IfcApproval`,
/// an empty, duplicated or out-of-type `RelatedObjects`; a model that binds
/// no single known release ([`ApprovalError::MultipleSchemas`],
/// [`ApprovalError::UnsupportedSchema`]), and an IFC2X3 model. Nothing is
/// staged on an error.
pub fn associate_approval(
    tx: &mut Transaction,
    model: &Model,
    draft: ApprovalAssociationDraft<'_>,
) -> ApprovalResult<EntityId> {
    associate(tx, model, draft, None)
}

/// [`associate_approval`] with a caller-supplied `IfcOwnerHistory`, which
/// IFC2X3 requires on every `IfcRoot`.
///
/// `owner_history` must be in the model or staged earlier on `tx`, and must
/// be an `IfcOwnerHistory`; one is never invented here (build it with
/// `ifc-author`). In IFC4 and IFC4X3 the reference fills the optional slot.
///
/// # Errors
///
/// Those of [`associate_approval`] except the IFC2X3 refusal, and
/// [`ApprovalError::UnknownEntity`] for an `owner_history` that does not
/// resolve or [`ApprovalError::AuthoringReferenceType`] for one that is not
/// an `IfcOwnerHistory`. Nothing is staged on an error.
pub fn associate_approval_with_owner_history(
    tx: &mut Transaction,
    model: &Model,
    draft: ApprovalAssociationDraft<'_>,
    owner_history: EntityId,
) -> ApprovalResult<EntityId> {
    associate(tx, model, draft, Some(owner_history))
}

/// Stage an `IfcRelAssociatesApproval`; `None` leaves `OwnerHistory` `$`.
fn associate(
    tx: &mut Transaction,
    model: &Model,
    draft: ApprovalAssociationDraft<'_>,
    owner_history: Option<EntityId>,
) -> ApprovalResult<EntityId> {
    let layout = bind(model)?;
    if Guid::parse(draft.global_id).is_none() {
        return Err(ApprovalError::AuthoringInvalid {
            entity: ASSIGNMENT,
            attribute: "GlobalId",
            value: draft.global_id.into(),
        });
    }
    validate_target_in(
        layout.schema(),
        tx,
        model,
        draft.relating_approval,
        APPROVAL,
    )?;
    if draft.related_objects.is_empty() {
        return Err(ApprovalError::AuthoringInvalid {
            entity: ASSIGNMENT,
            attribute: "RelatedObjects",
            value: "empty SET [1:?]".into(),
        });
    }
    let mut seen = HashSet::new();
    for &target in draft.related_objects {
        if !seen.insert(target) {
            return Err(ApprovalError::AuthoringInvalid {
                entity: ASSIGNMENT,
                attribute: "RelatedObjects",
                value: format!("duplicate {target}"),
            });
        }
        let actual =
            final_type(tx, model, target).ok_or(ApprovalError::UnknownEntity { id: target })?;
        if !layout.is_definition(actual) {
            return Err(ApprovalError::AuthoringReferenceType {
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
            ("RelatingApproval", Value::Ref(draft.relating_approval)),
        ],
    )?;
    Ok(tx.create(record))
}

#[allow(clippy::too_many_arguments)]
fn validate_set(
    schema: &Schema,
    tx: &Transaction,
    model: &Model,
    kind: &'static str,
    attribute: &'static str,
    targets: &[EntityId],
    expected: &'static str,
    disallow: Option<EntityId>,
) -> ApprovalResult<()> {
    if targets.is_empty() {
        return Err(ApprovalError::AuthoringInvalid {
            entity: kind,
            attribute,
            value: "empty SET [1:?]".into(),
        });
    }
    let mut seen = HashSet::new();
    for &target in targets {
        if !seen.insert(target) {
            return Err(ApprovalError::AuthoringInvalid {
                entity: kind,
                attribute,
                value: format!("duplicate {target}"),
            });
        }
        if Some(target) == disallow {
            return Err(ApprovalError::AuthoringInvalid {
                entity: kind,
                attribute,
                value: format!("self reference {target}"),
            });
        }
        validate_target_in(schema, tx, model, target, expected)?;
    }
    Ok(())
}

/// Fail unless `target` resolves, in the model or staged on `tx`, to a type
/// `schema` accepts as `expected`.
fn validate_target_in(
    schema: &Schema,
    tx: &Transaction,
    model: &Model,
    target: EntityId,
    expected: &'static str,
) -> ApprovalResult<()> {
    let actual =
        final_type(tx, model, target).ok_or(ApprovalError::UnknownEntity { id: target })?;
    if schema.accepts_type(expected, actual) {
        Ok(())
    } else {
        Err(ApprovalError::AuthoringReferenceType {
            target,
            expected,
            actual: actual.into(),
        })
    }
}

fn final_type<'a>(tx: &'a Transaction, model: &'a Model, id: EntityId) -> Option<&'a str> {
    for edit in tx.edits().iter().rev() {
        match edit {
            Edit::Create {
                id: edit_id,
                entity,
            } if *edit_id == id => return Some(&entity.type_name),
            Edit::Remove { id: edit_id } if *edit_id == id => return None,
            Edit::Retype {
                id: edit_id,
                type_name,
            } if *edit_id == id => return Some(type_name),
            _ => {}
        }
    }
    model.get(id).map(|entity| entity.type_name.as_ref())
}

fn text(value: &str) -> Value {
    Value::Text(Arc::from(value))
}
fn optional_text(value: Option<&str>) -> Value {
    value.map_or(Value::Null, text)
}
fn optional_ref(value: Option<EntityId>) -> Value {
    value.map_or(Value::Null, Value::Ref)
}
fn refs(values: &[EntityId]) -> Value {
    Value::List(values.iter().copied().map(Value::Ref).collect())
}
