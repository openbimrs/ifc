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
//! `IFCLABEL('x')` in an `IfcValue` slot is legal because `IfcLabel` is a
//! member of `IfcSimpleValue`, which is a member of `IfcValue`.

use ifc_schema::{Schema, TypeKind};

/// Bounded so a cyclic or pathological SELECT cannot hang a validation run.
const MAX_SELECT_DEPTH: usize = 32;

/// Whether `candidate` is reachable as a member of SELECT `type_name`.
///
/// Returns `None` when `type_name` is not a SELECT, so a caller can tell
/// "not a member" from "not a select".
#[must_use]
pub fn accepts(schema: &Schema, type_name: &str, candidate: &str) -> Option<bool> {
    let TypeKind::Select(_) = &schema.type_def(type_name)?.kind else {
        return None;
    };
    let mut frontier = vec![type_name.to_string()];
    let mut seen: Vec<String> = Vec::new();
    for _ in 0..MAX_SELECT_DEPTH {
        let Some(current) = frontier.pop() else { break };
        if seen.iter().any(|name| name.eq_ignore_ascii_case(&current)) {
            continue;
        }
        seen.push(current.clone());
        let Some(definition) = schema.type_def(&current) else {
            // An entity or primitive leaf: it matches only by name.
            continue;
        };
        match &definition.kind {
            TypeKind::Select(members) => {
                for member in members {
                    if member.eq_ignore_ascii_case(candidate) {
                        return Some(true);
                    }
                    frontier.push(member.clone());
                }
            }
            TypeKind::Defined(target) => {
                if target.eq_ignore_ascii_case(candidate) {
                    return Some(true);
                }
                frontier.push(target.clone());
            }
            TypeKind::Enumeration(_) => {}
        }
    }
    Some(false)
}
