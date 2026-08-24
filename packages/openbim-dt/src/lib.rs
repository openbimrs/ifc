//! `openbim-dt` — ISO 23387 data templates.
//!
//! # What this is
//!
//! The concept vocabulary that describes *properties themselves*: property
//! definitions, groups of properties, quantity kinds, dimensions, units,
//! object types, and the reference machinery binding them to external
//! dictionaries such as bSDD.
//!
//! # Why it is a crate and not part of `openbim-loin`
//!
//! Because LOIN does not own it. The ISO 7817-3 schema *imports* the ISO 23387
//! namespace and uses its types throughout — `PropertyType`, `ConceptType`,
//! `ReferenceType`, `GroupOfPropertiesType`. A future bSDD client needs the
//! same vocabulary and has no business depending on LOIN to get it.
//!
//! Practical consequence: `openbim`'s `loin` feature implies `dt`, but not the
//! reverse.
//!
//! # Status
//!
//! **Reserved — no implementation.** This crate is published to establish the
//! name and the layering. It parses nothing yet.
//!
//! The ISO 23387 XSD is **not vendored** here. Redistribution of ISO schemas
//! is not established by having a copy, so the schema is referenced out of
//! tree and types are written from it — the same discipline `ifc-schema`
//! applies to the EXPRESS schemas.

#![forbid(unsafe_code)]

/// The XML namespace ISO 23387 edition 2 declares.
///
/// Recorded now because namespace identity is how a reader tells this
/// vocabulary from a look-alike. Draft copies of this schema in circulation
/// still carry the `http://tempuri.org/XMLSchema.xsd` placeholder; a document
/// using that is **not** conformant and must not be accepted silently.
pub const NAMESPACE: &str = "https://standards.iso.org/iso/23387/ed-2/en/";

/// A placeholder namespace seen in pre-release drafts of the ISO 23387 XSD.
///
/// Kept as a named constant so that recognising it can produce a specific
/// diagnostic rather than a generic "unknown namespace" error.
pub const DRAFT_PLACEHOLDER_NAMESPACE: &str = "http://tempuri.org/XMLSchema.xsd";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn namespaces_are_distinct() {
        assert_ne!(NAMESPACE, DRAFT_PLACEHOLDER_NAMESPACE);
        assert!(NAMESPACE.starts_with("https://standards.iso.org/"));
    }
}
