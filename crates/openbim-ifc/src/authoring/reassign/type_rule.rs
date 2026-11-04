//! The type rules of a class and the type class each pairs with (#636).
//!
//! A class falls under a type rule when it, or a supertype, declares a
//! `WHERE` rule labelled `CorrectTypeAssigned` or `CorrectStyleAssigned`
//! that checks the type object of `IsTypedBy`. The bundled tables carry
//! rule labels, not expressions, so the type class a rule requires comes
//! from the `{RuleOwner}Type` convention (`IfcWall` and `IfcWallType`),
//! which `type_rule/tests.rs` checks against the normative EXPRESS of every
//! release. IFC2X3 declares no such rule, so nothing pairs there.

use ifc_schema::{EntityDef, Schema};

/// The `WHERE` rule labels that check a typed object's class against its
/// type object.
const TYPE_RULES: [&str; 2] = ["CorrectTypeAssigned", "CorrectStyleAssigned"];

/// Rules with a type-rule label that do not check the type object:
/// `IfcEvent.CorrectTypeAssigned` checks `EventTriggerType` (#670).
const NOT_TYPE_RULES: [(&str, &str); 1] = [("IfcEvent", "CorrectTypeAssigned")];

/// A type rule: the class that declares it and the type class it requires.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct TypeRule<'s> {
    pub(super) owner: &'s str,
    pub(super) label: &'s str,
    pub(super) type_class: &'s str,
}

impl TypeRule<'_> {
    /// `Owner.Label`, as the rule is named in a refusal.
    pub(super) fn name(&self) -> String {
        format!("{}.{}", self.owner, self.label)
    }
}

/// The label of the type rule `definition` declares itself.
fn own_label(definition: &EntityDef) -> Option<&str> {
    definition
        .where_rules
        .iter()
        .map(|rule| rule.label.as_str())
        .find(|label| {
            TYPE_RULES.contains(label)
                && !NOT_TYPE_RULES.contains(&(definition.name.as_str(), *label))
        })
}

/// The type rule `class` falls under: that of the nearest class, `class`
/// first, that declares one. `Err` names the gap when the owner has no
/// `{Owner}Type` that is an `IfcTypeObject` in this schema.
pub(super) fn type_rule<'s>(
    schema: &'s Schema,
    class: &str,
) -> Result<Option<TypeRule<'s>>, String> {
    let Some((owner, label)) = std::iter::once(class)
        .chain(schema.supertypes(class))
        .filter_map(|name| schema.entity(name))
        .find_map(|definition| own_label(definition).map(|label| (definition, label)))
    else {
        return Ok(None);
    };
    let name = format!("{}Type", owner.name);
    match schema.entity(&name) {
        Some(type_def) if schema.is_a(&type_def.name, "IfcTypeObject") => Ok(Some(TypeRule {
            owner: &owner.name,
            label,
            type_class: &type_def.name,
        })),
        _ => Err(format!(
            "{}.{label} names no {name} in {}",
            owner.name,
            schema.name()
        )),
    }
}

/// The occurrence class that pairs with the type class `type_class`: the
/// nearest class at or above it, named `{S}Type`, whose stem `S` is a
/// declared, non-abstract class that declares its own type rule.
#[cfg(test)] // the pairing of #636 follows
pub(super) fn occurrence_class<'s>(schema: &'s Schema, type_class: &str) -> Option<&'s str> {
    std::iter::once(type_class)
        .chain(schema.supertypes(type_class))
        .find_map(|name| {
            let stem = name.strip_suffix("Type")?;
            let definition = schema.entity(stem)?;
            (!definition.abstract_ && own_label(definition).is_some())
                .then_some(definition.name.as_str())
        })
}

#[cfg(test)]
mod tests;
