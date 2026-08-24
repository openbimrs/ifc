//! `openbim-idm` — ISO 29481-3 idmXML (Information Delivery Manual).
//!
//! # What this is
//!
//! The machine-readable half of the IDM family. ISO 29481-1 defines the
//! methodology and 29481-2 the interaction framework (BPMN process maps);
//! **part 3** specifies the XML schema for exchange requirements, use cases
//! and business context — the part that can be parsed.
//!
//! # Why the crate is `openbim-idm` and the alias is `idmxml`
//!
//! `idm` on crates.io is taken by an unrelated, actively maintained project
//! ("Implicit Data Markup"). `idmxml` is in any case the more precise name:
//! it distinguishes part 3 from the process-map halves of IDM that this crate
//! does not implement.
//!
//! # Known schema defects to preserve
//!
//! Prior work against the official Annex B XSDs recorded two inconsistencies
//! that any implementation has to take a position on:
//!
//! - the root exchange requirement is **optional** in the schema while the
//!   normative prose requires exactly one;
//! - several identity-constraint XPath expressions appear to be defective.
//!
//! These are not bugs to silently work around — a reader's behaviour on each
//! is an interoperability decision and must be documented when implemented.
//!
//! # Status
//!
//! **Reserved — no implementation.** Published to establish the name.
//!
//! A working lossless ISO 29481-3 codec exists in the private `poing`
//! repository and is intended to move here; that port is deliberately not part
//! of this release. The Annex B XSDs are **not vendored** — their
//! redistribution licence is unresolved.

#![forbid(unsafe_code)]

/// Document kinds ISO 29481-3 defines, each with its own schema.
///
/// Modelled as an enum rather than free strings because "which schema does
/// this document follow" is the first branch every reader must take, and the
/// set is closed by the standard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DocumentKind {
    /// An exchange requirement: what information is handed over.
    ExchangeRequirement,
    /// A use case description.
    UseCase,
    /// A business context map.
    BusinessContextMap,
    /// Authoring/provenance metadata.
    Authoring,
}

impl DocumentKind {
    /// Every kind, in schema order.
    pub const ALL: [DocumentKind; 4] = [
        DocumentKind::ExchangeRequirement,
        DocumentKind::UseCase,
        DocumentKind::BusinessContextMap,
        DocumentKind::Authoring,
    ];
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_kinds_are_distinct() {
        let mut seen = std::collections::HashSet::new();
        for k in DocumentKind::ALL {
            assert!(seen.insert(k), "duplicate document kind: {k:?}");
        }
        assert_eq!(seen.len(), 4);
    }
}
