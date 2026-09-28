//! SELECT membership.
//!
//! # Why a SELECT is checked structurally, not by name
//!
//! An EXPRESS `SELECT` lists alternative types, which may themselves be
//! selects, defined types, or entities:
//!
//! ```text
//! TYPE IfcValue = SELECT (IfcMeasureValue, IfcSimpleValue, IfcDerivedMeasureValue);
//! ```
//!
//! So membership is a graph walk, not a list lookup. A typed value
//! `IFCMONETARYMEASURE(1.0)` in an `IfcAppliedValueSelect` slot is legal
//! because `IfcMonetaryMeasure` is a member of `IfcDerivedMeasureValue`,
//! which is a member of `IfcValue`, which is a member of
//! `IfcAppliedValueSelect`.
//!
//! # The walk must be bounded by visits, not by iterations
//!
//! A previous version capped the *loop* at 32 iterations. IFC4's value
//! selects are wide -- `IfcDerivedMeasureValue` alone lists 60 members -- so
//! the budget was exhausted while the frontier still held unexplored nodes,
//! and the function returned `false` for types that are perfectly legal. A
//! bound that turns "not yet searched" into "not a member" produces confident
//! false accusations, which is worse than no check.
//!
//! The bound is now on distinct types visited, which is bounded by the schema
//! and cannot be exhausted by a legal file.
//!
//! # Only nested SELECTs are walked, never defined types
//!
//! ISO 10303-21:2016 §12.1.8 requires the keyword of a typed parameter to
//! name the value's own type, and that type to be one of the types listed by
//! the SELECT or, recursively, by a SELECT nested in it. A defined type in the list is a leaf: `IfcColourOrFactor` lists
//! `IfcNormalisedRatioMeasure`, whose underlying type is `IfcRatioMeasure`,
//! and `IFCRATIOMEASURE(0.5)` is not a member -- a ratio is not known to be
//! normalised. A previous walk descended into defined types and accepted it.
//!
//! A type that *aliases* a SELECT is encoded as that SELECT, though
//! (§12.1.8, EXAMPLE 2): given `TYPE Computed_Load = Number_Or_Flag` where
//! `Number_Or_Flag` is a SELECT, a `Computed_Load` value is written
//! `COMPUTED_LOAD(PLAIN_NUMBER(1.5))`, its parameter itself typed.
//! [`resolve_select`] follows such aliases.

use std::collections::BTreeSet;

use ifc_schema::{Schema, TypeKind};

/// Upper bound on distinct types visited in one membership query.
///
/// IFC4's largest reachable select closure is a few hundred types; this is
/// slack above that, and exists only so a malformed or cyclic schema cannot
/// hang a validation run. Cycles are already handled by the visited set.
const MAX_VISITED: usize = 4096;

/// How many defined-type alias hops are followed to find a SELECT.
///
/// No bundled IFC release aliases a SELECT at all; this only stops a
/// malformed cyclic table from looping.
const MAX_ALIAS_HOPS: usize = 16;

/// The SELECT a declared type is, following defined-type aliases.
///
/// `IfcValue` -> `IfcValue`; `Computed_Load` -> `Number_Or_Flag` for
/// `TYPE Computed_Load = Number_Or_Flag`; `IfcLabel` -> `None`. A type
/// aliasing an aggregate or a primitive is not a SELECT, and neither is an
/// entity or a name the schema does not declare.
#[must_use]
pub fn resolve_select(schema: &Schema, type_name: &str) -> Option<String> {
    let mut current = type_name.trim().to_string();
    for _ in 0..MAX_ALIAS_HOPS {
        match &schema.type_def(&current)?.kind {
            TypeKind::Select(_) => return Some(current),
            TypeKind::Defined(target) => current = target.trim().to_string(),
            TypeKind::Enumeration(_) => return None,
            _ => return None,
        }
    }
    None
}

/// Whether `candidate` is named in the select-list of SELECT `type_name`,
/// or of a SELECT nested in it.
///
/// Returns `None` when `type_name` is not a SELECT, even through aliases,
/// so a caller can tell "not a member" from "not a select".
#[must_use]
pub fn accepts(schema: &Schema, type_name: &str, candidate: &str) -> Option<bool> {
    let select = resolve_select(schema, type_name)?;
    let mut frontier = vec![select.to_ascii_uppercase()];
    let mut seen: BTreeSet<String> = BTreeSet::new();
    while let Some(current) = frontier.pop() {
        if !seen.insert(current.clone()) {
            continue;
        }
        if seen.len() > MAX_VISITED {
            // Refuse to answer rather than answer wrongly: an exhausted walk
            // has not proven the candidate absent.
            return None;
        }
        // Entities, defined types and enumerations are leaves: they match
        // only by name, which the push site already checked. Only a nested
        // SELECT contributes members of its own (§12.1.8 NOTE 1).
        let Some(TypeKind::Select(members)) =
            schema.type_def(&current).map(|definition| &definition.kind)
        else {
            continue;
        };
        for member in members {
            if member.eq_ignore_ascii_case(candidate) {
                return Some(true);
            }
            frontier.push(member.to_ascii_uppercase());
        }
    }
    Some(false)
}

/// The entity alternatives a SELECT offers, found by walking nested SELECTs.
struct EntityMembers {
    /// Entities named anywhere in the closure. An instance of any of them,
    /// or of a subtype, is a member.
    entities: Vec<String>,
    /// Whether the closure also offers a defined type or enumeration, i.e.
    /// a value that is not an entity reference.
    values: bool,
    /// Whether a member names nothing the schema declares.
    unknown: bool,
}

/// Walks the closure of SELECT `type_name` for its entity alternatives.
///
/// `None` when `type_name` is not a SELECT, or the visit bound was hit.
fn entity_members(schema: &Schema, type_name: &str) -> Option<EntityMembers> {
    let select = resolve_select(schema, type_name)?;
    let mut members = EntityMembers {
        entities: Vec::new(),
        values: false,
        unknown: false,
    };
    let mut frontier = vec![select.to_ascii_uppercase()];
    let mut seen: BTreeSet<String> = BTreeSet::new();
    while let Some(current) = frontier.pop() {
        if !seen.insert(current.clone()) {
            continue;
        }
        if seen.len() > MAX_VISITED {
            return None;
        }
        if schema.entity(&current).is_some() {
            members.entities.push(current);
            continue;
        }
        match schema.type_def(&current).map(|definition| &definition.kind) {
            Some(TypeKind::Select(nested)) => {
                frontier.extend(nested.iter().map(|member| member.to_ascii_uppercase()));
            }
            Some(TypeKind::Defined(_) | TypeKind::Enumeration(_)) => members.values = true,
            // Unknown declaration forms fail closed like undeclared names.
            Some(_) | None => members.unknown = true,
        }
    }
    Some(members)
}

/// Whether an instance of `entity_type` may be referenced from a slot of
/// SELECT `type_name`.
///
/// It may when it is, or inherits from, an entity anywhere in the SELECT's
/// closure. `None` when `type_name` is not a SELECT, or when the answer
/// would be "no" but some member names nothing the schema declares -- that
/// member might have admitted it.
#[must_use]
pub fn admits_entity(schema: &Schema, type_name: &str, entity_type: &str) -> Option<bool> {
    let members = entity_members(schema, type_name)?;
    if members
        .entities
        .iter()
        .any(|entity| schema.is_a(entity_type, entity))
    {
        Some(true)
    } else if members.unknown {
        None
    } else {
        Some(false)
    }
}

/// Whether every alternative of SELECT `type_name` is an entity, so that
/// only an entity reference can fill its slot.
///
/// `None` when `type_name` is not a SELECT or its closure cannot be fully
/// resolved.
#[must_use]
pub fn admits_only_entities(schema: &Schema, type_name: &str) -> Option<bool> {
    let members = entity_members(schema, type_name)?;
    (!members.unknown).then_some(!members.values)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An entity alternative admits its subtypes; nested SELECTs are walked.
    #[test]
    fn entity_membership_walks_nested_selects_and_subtypes() {
        let schema = ifc_schema::ifc4();
        // IfcMaterialSelect lists IfcMaterialDefinition; IfcMaterial is a subtype.
        assert_eq!(
            admits_entity(schema, "IfcMaterialSelect", "IfcMaterial"),
            Some(true)
        );
        assert_eq!(
            admits_entity(schema, "IfcMaterialSelect", "IfcWall"),
            Some(false)
        );
        // IfcDefinitionSelect -> IfcObjectDefinition -> ... -> IfcWall.
        assert_eq!(
            admits_entity(schema, "IfcDefinitionSelect", "IfcWall"),
            Some(true)
        );
        // IfcCurveFontOrScaledCurveFontSelect -> IfcCurveStyleFontSelect
        // -> IfcCurveStyleFont: an entity one SELECT down.
        assert_eq!(
            admits_entity(
                schema,
                "IfcCurveFontOrScaledCurveFontSelect",
                "IfcCurveStyleFont"
            ),
            Some(true)
        );
        assert_eq!(admits_entity(schema, "IfcLabel", "IfcWall"), None);
    }

    /// Entity-only SELECTs are told apart from ones that also take values.
    #[test]
    fn entity_only_selects_are_recognised() {
        let schema = ifc_schema::ifc4();
        assert_eq!(admits_only_entities(schema, "IfcActorSelect"), Some(true));
        assert_eq!(admits_only_entities(schema, "IfcValue"), Some(false));
        assert_eq!(
            admits_only_entities(schema, "IfcPropertySetDefinitionSelect"),
            Some(false)
        );
    }

    /// A member two hops down a wide select must be found.
    ///
    /// `IfcAppliedValueSelect -> IfcValue -> IfcDerivedMeasureValue`, where
    /// the last lists 60 members. This is the case an iteration-capped walk
    /// got wrong, and it wrongly rejected every monetary cost value in the
    /// fixture corpus.
    #[test]
    fn a_member_behind_a_wide_select_is_found() {
        let schema = ifc_schema::ifc4();
        assert_eq!(
            accepts(schema, "IfcAppliedValueSelect", "IfcMonetaryMeasure"),
            Some(true),
        );
    }

    /// The same, for the select used by property values.
    #[test]
    fn a_derived_measure_is_a_member_of_ifc_value() {
        let schema = ifc_schema::ifc4();
        assert_eq!(
            accepts(schema, "IfcValue", "IfcVolumetricFlowRateMeasure"),
            Some(true),
        );
    }

    /// A genuine non-member is still rejected, so the fix is not "say yes".
    #[test]
    fn a_non_member_is_still_rejected() {
        let schema = ifc_schema::ifc4();
        assert_eq!(accepts(schema, "IfcValue", "IfcWall"), Some(false));
    }

    /// Every SELECT a bundled schema uses in an attribute slot is walked to
    /// completion under the visit bound, so a legal file cannot exhaust it.
    ///
    /// This is what lets the bound be a crate constant rather than a
    /// caller-supplied budget: it is a property of the bundled schemas, and
    /// a non-member query walks a select's entire closure.
    #[test]
    fn every_bundled_select_closure_completes_within_the_visit_bound() {
        for schema in [
            ifc_schema::ifc2x3(),
            ifc_schema::ifc4(),
            ifc_schema::ifc4x3(),
        ] {
            let mut selects = BTreeSet::new();
            for entity in schema.entity_names() {
                for attribute in schema.attributes(entity) {
                    if let Some(definition) = schema.type_def(&attribute.type_name) {
                        if matches!(definition.kind, TypeKind::Select(_)) {
                            selects.insert(attribute.type_name.clone());
                        }
                    }
                }
            }
            assert!(selects.len() > 20, "{}: {selects:?}", schema.name());
            for select in &selects {
                assert_eq!(
                    accepts(schema, select, "NOT_A_DECLARED_TYPE"),
                    Some(false),
                    "{}: the walk over {select} did not complete",
                    schema.name()
                );
            }
        }
    }

    /// A type that is not a select is distinguishable from a non-member.
    #[test]
    fn a_non_select_returns_none() {
        let schema = ifc_schema::ifc4();
        assert_eq!(accepts(schema, "IfcLengthMeasure", "REAL"), None);
    }
}
