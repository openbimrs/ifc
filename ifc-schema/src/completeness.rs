//! Prove a hand-written entity inventory is complete against the normative
//! EXPRESS schema.
//!
//! # Why this exists
//!
//! Several crates publish a `const` list describing itself as the complete
//! inventory for one IFC schema, and assert its own length in a test. That
//! gate cannot fail: trimming the list trims the expectation with it. The
//! material inventory was short one entity for exactly this reason.
//!
//! # What this checks
//!
//! An inventory declares the roots it covers. Every entity the schema
//! declares as a descendant of those roots must appear in the inventory.
//! The schema supplies the expectation, so deleting a name makes the
//! check fail rather than lowering the bar.
//!
//! This proves the inventory names every entity. It does not prove the
//! crate implements them; that is the job of each crate behaviour tests.

use crate::Schema;
use std::collections::BTreeSet;

/// What an inventory got wrong, in schema terms.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InventoryGap {
    /// Declared by the schema under a covered root, absent from the list.
    pub missing: BTreeSet<String>,
    /// Named by the list, not declared by the schema under any root.
    pub unknown: BTreeSet<String>,
}

impl InventoryGap {
    /// Whether the inventory matched the schema exactly.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.missing.is_empty() && self.unknown.is_empty()
    }
}

impl std::fmt::Display for InventoryGap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if !self.missing.is_empty() {
            writeln!(f, "declared by the schema but absent from the inventory:")?;
            for name in &self.missing {
                writeln!(f, "    {name}")?;
            }
        }
        if !self.unknown.is_empty() {
            writeln!(f, "named by the inventory but not declared under any root:")?;
            for name in &self.unknown {
                writeln!(f, "    {name}")?;
            }
        }
        Ok(())
    }
}

/// Every entity the schema declares at or below `roots`, upper-cased.
///
/// Walks the declared supertype chain of every entity rather than the
/// subtype lists, because an entity can be reached through a chain whose
/// intermediate links are abstract.
#[must_use]
pub fn descendants_of(schema: &Schema, roots: &[&str]) -> BTreeSet<String> {
    let wanted: BTreeSet<String> = roots.iter().map(|r| r.to_ascii_uppercase()).collect();
    let mut found = BTreeSet::new();
    for name in schema.entity_names() {
        let upper = name.to_ascii_uppercase();
        if wanted.contains(&upper) {
            found.insert(upper);
            continue;
        }
        if schema
            .supertypes(name)
            .iter()
            .any(|s| wanted.contains(&s.to_ascii_uppercase()))
        {
            found.insert(upper);
        }
    }
    found
}

/// Compare a published inventory against what the schema declares.
///
/// `roots` are the entities the inventory claims to cover, including every
/// entity below them. Names are compared case-insensitively.
#[must_use]
pub fn audit_inventory(schema: &Schema, roots: &[&str], inventory: &[&str]) -> InventoryGap {
    let expected = descendants_of(schema, roots);
    let actual: BTreeSet<String> = inventory.iter().map(|e| e.to_ascii_uppercase()).collect();
    InventoryGap {
        missing: expected.difference(&actual).cloned().collect(),
        unknown: actual.difference(&expected).cloned().collect(),
    }
}
