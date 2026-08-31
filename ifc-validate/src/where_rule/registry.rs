//! Which WHERE rules this validator actually evaluates.
//!
//! # The honesty problem this solves
//!
//! IFC4 declares hundreds of `WHERE` rules as EXPRESS expressions. This
//! validator does not have an EXPRESS expression evaluator, so it cannot
//! check most of them. The tempting design is to check the ones it can and
//! stay quiet about the rest -- which produces a clean report for a file
//! nobody fully checked, and a user who believes it.
//!
//! Instead every rule is registered with an explicit state. Supported rules
//! are evaluated; unsupported ones are *reported* as unsupported. A caller
//! can therefore distinguish "this file is conformant" from "this file did
//! not trip the subset we implement".
//!
//! # Why a registry rather than a list of implemented functions
//!
//! Rules get implemented over time. If support were implicit in which
//! functions exist, nothing would tell a reader which rules are missing --
//! the absence of code is invisible. The registry makes the gap a data
//! structure that can be counted, printed, and tested against.

/// Whether this validator evaluates a given rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Support {
    /// Implemented and evaluated on every run.
    Implemented,
    /// Declared by the schema, not evaluated here.
    ///
    /// Carries why, so the report can say something better than "no".
    Unsupported(&'static str),
}

/// One registered rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuleEntry {
    /// Stable rule id, e.g. `IfcRoot.WR1` or `global.IfcSingleProjectInstance`.
    pub id: &'static str,
    /// The entity the rule constrains, or `None` for a global rule.
    pub entity: Option<&'static str>,
    /// Whether it is evaluated.
    pub support: Support,
}

/// Reason strings, shared so the same gap reads identically everywhere.
const NEEDS_EXPRESSIONS: &str = "requires an EXPRESS expression evaluator";
const NEEDS_BOUNDS: &str = "requires aggregate bounds, which the schema parser does not retain";
const NEEDS_INVERSES: &str =
    "not implemented uniformly: IFC2X3 requires INVERSE relationship semantics, which validation does not derive";
const NEEDS_GEOMETRY: &str = "requires geometric evaluation, which validation does not perform";

/// Every rule this validator knows about, implemented or not.
///
/// Deliberately not exhaustive over every bundled IFC release: claiming to
/// enumerate all rules would be its own dishonesty. It covers selected
/// high-value predicates plus representative unsupported categories.
pub const RULES: &[RuleEntry] = &[
    RuleEntry {
        id: "global.IfcSingleProjectInstance",
        entity: None,
        support: Support::Implemented,
    },
    // IfcRoot constrains GlobalId with `UNIQUE UR1`, not a WHERE rule. It is
    // registered here under its global id rather than as `IfcRoot.UR1`: the
    // check is file-wide, and two entries for one check would double-report.
    RuleEntry {
        id: "global.UniqueGlobalId",
        entity: None,
        support: Support::Implemented,
    },
    RuleEntry {
        id: "IfcRelDefinesByProperties.NoRelatedTypeObject",
        entity: Some("IfcRelDefinesByProperties"),
        support: Support::Implemented,
    },
    RuleEntry {
        id: "IfcExternalReference.WR1",
        entity: Some("IfcExternalReference"),
        support: Support::Implemented,
    },
    RuleEntry {
        id: "IfcRelSequence.WR1",
        entity: Some("IfcRelSequence"),
        support: Support::Implemented,
    },
    RuleEntry {
        id: "IfcRelSequence.AvoidInconsistentSequence",
        entity: Some("IfcRelSequence"),
        support: Support::Implemented,
    },
    RuleEntry {
        id: "IfcRelAggregates.NoSelfReference",
        entity: Some("IfcRelAggregates"),
        support: Support::Implemented,
    },
    RuleEntry {
        id: "IfcRelNests.NoSelfReference",
        entity: Some("IfcRelNests"),
        support: Support::Implemented,
    },
    RuleEntry {
        id: "IfcMaterialLayer.NormalizedPriority",
        entity: Some("IfcMaterialLayer"),
        support: Support::Implemented,
    },
    RuleEntry {
        id: "IfcDocumentReference.WR1",
        entity: Some("IfcDocumentReference"),
        support: Support::Unsupported(NEEDS_INVERSES),
    },
    RuleEntry {
        id: "IfcRepresentationContextSameWCS",
        entity: None,
        support: Support::Unsupported(NEEDS_GEOMETRY),
    },
    RuleEntry {
        id: "IfcPolyLoop.WR21",
        entity: Some("IfcPolyLoop"),
        support: Support::Unsupported(NEEDS_BOUNDS),
    },
    RuleEntry {
        id: "IfcPhysicalSimpleQuantity.WR21",
        entity: Some("IfcQuantityLength"),
        support: Support::Unsupported(NEEDS_EXPRESSIONS),
    },
    RuleEntry {
        id: "IfcZone.WR1",
        entity: Some("IfcZone"),
        support: Support::Unsupported(NEEDS_EXPRESSIONS),
    },
];

/// The registered entry for `id`, if there is one.
#[must_use]
pub fn lookup(id: &str) -> Option<&'static RuleEntry> {
    RULES.iter().find(|entry| entry.id == id)
}

/// Every rule this validator does not evaluate.
pub fn unsupported() -> impl Iterator<Item = &'static RuleEntry> {
    RULES
        .iter()
        .filter(|entry| matches!(entry.support, Support::Unsupported(_)))
}

/// Every rule this validator does evaluate.
pub fn implemented() -> impl Iterator<Item = &'static RuleEntry> {
    RULES
        .iter()
        .filter(|entry| matches!(entry.support, Support::Implemented))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Rule ids must be unique: they are what callers suppress on.
    #[test]
    fn rule_ids_are_unique() {
        let mut ids: Vec<&str> = RULES.iter().map(|entry| entry.id).collect();
        ids.sort_unstable();
        let count = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), count, "duplicate rule id in the registry");
    }

    /// The registry must actually contain unevaluated rules.
    ///
    /// If this ever reports zero, either every IFC4 rule is implemented -- it
    /// is not -- or rules are being dropped from the registry instead of
    /// being marked unsupported, which is exactly the dishonesty the registry
    /// exists to prevent.
    #[test]
    fn the_registry_admits_what_it_cannot_check() {
        assert!(
            unsupported().count() > 0,
            "a validator claiming full WHERE-rule coverage is lying"
        );
    }

    #[test]
    fn unsupported_boundaries_remain_explicit() {
        let reasons: Vec<_> = unsupported()
            .filter_map(|entry| match entry.support {
                Support::Unsupported(reason) => Some(reason),
                Support::Implemented => None,
            })
            .collect();
        for required in ["aggregate bounds", "EXPRESS expression", "INVERSE"] {
            assert!(
                reasons.iter().any(|reason| reason.contains(required)),
                "missing explicit unsupported boundary for {required}: {reasons:?}"
            );
        }
    }
}
