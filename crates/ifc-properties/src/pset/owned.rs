//! Predefined property sets and templates in the model's release (#202).
//!
//! The predefined sets (`IfcDoorLiningProperties`, ...,
//! `IfcReinforcementDefinitionProperties`) and the templates
//! (`IfcPropertySetTemplate`, `IfcComplexPropertyTemplate`,
//! `IfcRelDefinesByTemplate`) are `IfcRoot`s, and their writers take no
//! model: they write the IFC4 layout with `OwnerHistory` `$`. That record
//! is valid IFC4 and IFC4X3, whose layouts of these entities are the same,
//! and never valid IFC2X3:
//!
//! ```text
//! IFC2X3_TC1   OwnerHistory : IfcOwnerHistory;
//!              IfcDoorLiningProperties     no LiningToPanelOffsetX/Y
//!              IfcWindowLiningProperties   no LiningOffset, LiningToPanelOffsetX/Y
//!              LiningThickness, TransomThickness, ... : IfcPositiveLengthMeasure
//!              no property templates at all
//! IFC4         OwnerHistory : OPTIONAL IfcOwnerHistory;
//! IFC4X3_ADD2  as IFC4 for every entity here
//! ```
//!
//! The `*_with_owner_history` variants take the model and a caller-supplied
//! `IfcOwnerHistory`. They bind the declared release through
//! `quantity/release.rs`, lay the record out by attribute name from its
//! table, refuse an entity the release does not declare
//! ([`PropertyError::EntityNotInSchema`]) and a value for an attribute it
//! does not declare ([`PropertyError::AuthoringNotInSchema`]), and check
//! each value against the type the release declares for it: a thickness
//! of zero is an `IfcNonNegativeLengthMeasure` in IFC4 and not an
//! `IfcPositiveLengthMeasure` in IFC2X3.

use ifc_model::{EntityId, Model, Transaction, Value};
use ifc_schema::{SchemaVersion, TypeKind};

use crate::quantity::release::{bind, layout, Layout};
use crate::{PropertyError, PropertyResult};

use super::authoring::optional_text;
use super::root_authoring::require_owner_history;

/// An `IfcRoot` record of this module, before a release lays it out: the
/// entity, its root text, and its own attributes by IFC4 name. The
/// GlobalId has been checked.
pub(super) struct Rooted<'a> {
    pub(super) entity: &'static str,
    pub(super) global_id: &'a str,
    pub(super) name: Option<&'a str>,
    pub(super) description: Option<&'a str>,
    pub(super) values: Vec<(&'static str, Value)>,
}

impl Rooted<'_> {
    fn named(self, owner_history: Option<EntityId>) -> Vec<(&'static str, Value)> {
        let mut named = vec![
            ("GlobalId", Value::Text(self.global_id.into())),
            (
                "OwnerHistory",
                owner_history.map_or(Value::Null, Value::Ref),
            ),
            ("Name", optional_text(self.name)),
            ("Description", optional_text(self.description)),
        ];
        named.extend(self.values);
        named
    }
}

/// Stage `rooted` in the IFC4 layout with `OwnerHistory` `$`: what the
/// writers that take no model wrote before #202, unchanged.
pub(super) fn stage_ifc4(tx: &mut Transaction, rooted: Rooted<'_>) -> PropertyResult<EntityId> {
    let entity = rooted.entity;
    let record = layout(SchemaVersion::Ifc4)?.named_record(entity, rooted.named(None))?;
    Ok(tx.create(record))
}

/// Stage `rooted` in `model`'s declared release, with `owner_history`.
///
/// Nothing is staged on an error.
pub(super) fn stage_owned(
    tx: &mut Transaction,
    model: &Model,
    rooted: Rooted<'_>,
    owner_history: EntityId,
) -> PropertyResult<EntityId> {
    let layout = bind(model)?;
    let entity = rooted.entity;
    if layout.schema().entity(entity).is_none_or(|e| e.abstract_) {
        return Err(PropertyError::EntityNotInSchema {
            entity,
            schema: layout.version(),
        });
    }
    for (attribute, value) in &rooted.values {
        check_value(layout, entity, attribute, value)?;
    }
    require_owner_history(tx, model, layout, entity, owner_history)?;
    let record = layout.named_record(entity, rooted.named(Some(owner_history)))?;
    Ok(tx.create(record))
}

/// Refuse `value` where the type the release declares for `attribute`
/// cannot hold it: a non-positive `IfcPositiveLengthMeasure`, or an
/// enumeration token the release's enumeration does not list. An attribute
/// the release does not declare is left to the layout, which refuses a
/// value for it.
fn check_value(
    layout: Layout,
    entity: &'static str,
    attribute: &'static str,
    value: &Value,
) -> PropertyResult<()> {
    let schema = layout.schema();
    let Some(declared) = schema
        .attributes(entity)
        .into_iter()
        .find(|found| found.name.eq_ignore_ascii_case(attribute))
    else {
        return Ok(());
    };
    let refused = || PropertyError::AuthoringInvalid {
        entity,
        attribute,
        value: format!(
            "{value:?} is not a {} in {:?}",
            declared.type_name,
            layout.version()
        ),
    };
    match value {
        Value::Real(number)
            if declared
                .type_name
                .eq_ignore_ascii_case("IfcPositiveLengthMeasure")
                && *number <= 0.0 =>
        {
            Err(refused())
        }
        Value::Enum(token) => match schema.type_def(&declared.type_name).map(|t| &t.kind) {
            Some(TypeKind::Enumeration(members))
                if !members.iter().any(|member| member == token.as_ref()) =>
            {
                Err(refused())
            }
            _ => Ok(()),
        },
        _ => Ok(()),
    }
}
