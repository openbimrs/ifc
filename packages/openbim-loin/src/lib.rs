//! `openbim-loin` — ISO 7817-3 / EN 17412-3 Level of Information Need.
//!
//! # What this is
//!
//! A machine-readable statement of *how much* information is required about
//! which objects, for a given purpose, at a given milestone, between a given
//! pair of actors: geometric detail, alphanumeric properties, and
//! documentation.
//!
//! EN 17412-1 defines the concepts in prose; part 3 is the exchange format.
//! Only part 3 is implementable, and it is what this crate targets.
//!
//! # Depends on ISO 23387
//!
//! The LOIN schema imports the ISO 23387 namespace for its property
//! vocabulary, so this crate depends on `openbim-dt`. That is a property of
//! the standard, not a design choice.
//!
//! # 🚨 The namespace is not final
//!
//! The draft schema carries, in a comment on line 2:
//!
//! > `Final LOIN Namespace will be specified after the review process`
//!
//! and declares `https://iso.org/2024/LOIN`, while an earlier draft declared
//! `https://iso.org/2022/LOIN`. Namespace migration is therefore a
//! *first-class* concern here, not an afterthought: reading must accept known
//! historical namespaces, and writing must target one explicitly rather than
//! defaulting to whatever was parsed.
//!
//! # Status
//!
//! **Reserved — no implementation.** Published to establish the name.
//!
//! The ISO XSD is **not vendored**. Both the ISO/CEN originals and the public
//! committee drafts are unlicensed for redistribution, and the schema is a
//! moving target; it is referenced out of tree instead.

#![forbid(unsafe_code)]

/// The namespace declared by the ISO 7817-3 draft schema (2024).
pub const NAMESPACE_2024: &str = "https://iso.org/2024/LOIN";

/// The namespace declared by the earlier EN 17412-3 committee draft (2022).
///
/// Retained because documents using it exist. Reading should accept it;
/// writing should not emit it.
pub const NAMESPACE_2022: &str = "https://iso.org/2022/LOIN";

/// Namespaces this crate recognises as LOIN, newest first.
///
/// Ordered so that a reader trying candidates in sequence prefers the current
/// one, and so that adding the final published namespace is a one-line change
/// at the front of the list.
pub const KNOWN_NAMESPACES: &[&str] = &[NAMESPACE_2024, NAMESPACE_2022];

/// Whether a namespace URI is a LOIN namespace this crate knows.
///
/// ```
/// use openbim_loin::{is_known_namespace, NAMESPACE_2024};
/// assert!(is_known_namespace(NAMESPACE_2024));
/// assert!(!is_known_namespace("https://example.invalid/LOIN"));
/// ```
#[must_use]
pub fn is_known_namespace(ns: &str) -> bool {
    KNOWN_NAMESPACES.contains(&ns)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_draft_namespaces_are_recognised() {
        assert!(is_known_namespace(NAMESPACE_2024));
        assert!(is_known_namespace(NAMESPACE_2022));
    }

    #[test]
    fn unknown_namespace_is_rejected() {
        assert!(!is_known_namespace(""));
        assert!(!is_known_namespace("https://iso.org/2099/LOIN"));
    }

    #[test]
    fn current_namespace_is_preferred_first() {
        assert_eq!(KNOWN_NAMESPACES.first(), Some(&NAMESPACE_2024));
    }
}
