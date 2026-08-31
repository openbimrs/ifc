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

use std::collections::BTreeSet;

use ifc_schema::{Schema, TypeKind};

/// Upper bound on distinct types visited in one membership query.
///
/// IFC4's largest reachable select closure is a few hundred types; this is
/// slack above that, and exists only so a malformed or cyclic schema cannot
/// hang a validation run. Cycles are already handled by the visited set.
const MAX_VISITED: usize = 4096;

/// Whether `candidate` is reachable as a member of SELECT `type_name`.
///
/// Returns `None` when `type_name` is not a SELECT, so a caller can tell
/// "not a member" from "not a select".
#[must_use]
pub fn accepts(schema: &Schema, type_name: &str, candidate: &str) -> Option<bool> {
    let TypeKind::Select(_) = &schema.type_def(type_name)?.kind else {
        return None;
    };
    let mut frontier = vec![type_name.to_ascii_uppercase()];
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
        let Some(definition) = schema.type_def(&current) else {
            // An entity or primitive leaf: it matches only by name, which the
            // push site already checked.
            continue;
        };
        match &definition.kind {
            TypeKind::Select(members) => {
                for member in members {
                    if member.eq_ignore_ascii_case(candidate) {
                        return Some(true);
                    }
                    frontier.push(member.to_ascii_uppercase());
                }
            }
            TypeKind::Defined(target) => {
                if target.eq_ignore_ascii_case(candidate) {
                    return Some(true);
                }
                frontier.push(target.trim().to_ascii_uppercase());
            }
            TypeKind::Enumeration(_) => {}
        }
    }
    Some(false)
}

#[cfg(test)]
mod tests {
    use super::*;

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

    /// A type that is not a select is distinguishable from a non-member.
    #[test]
    fn a_non_select_returns_none() {
        let schema = ifc_schema::ifc4();
        assert_eq!(accepts(schema, "IfcLengthMeasure", "REAL"), None);
    }
}
