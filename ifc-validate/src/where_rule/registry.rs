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

use ifc_schema::{Schema, SchemaVersion};

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
///
/// The entry is the rule's whole scope: it applies to instances of
/// [`Self::entity`] *and of its subtypes*, and only under the
/// [`Self::releases`] whose EXPRESS declares it under this id. The engine
/// reads both from here, so a rule cannot run on a narrower set of
/// instances, or under more releases, than its entry states.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuleEntry {
    /// Stable rule id: `<declaring entity>.<label>` exactly as the schema
    /// writes it (e.g. `IfcExternalReference.WR1`), or `global.<name>` for a
    /// file-wide rule.
    pub id: &'static str,
    /// The entity declaring the rule, or `None` for a global rule.
    ///
    /// Instances of its subtypes are constrained too: EXPRESS WHERE rules
    /// are inherited.
    pub entity: Option<&'static str>,
    /// The releases whose EXPRESS declares this rule under [`Self::id`].
    pub releases: &'static [SchemaVersion],
    /// Whether it is evaluated.
    pub support: Support,
}

impl RuleEntry {
    /// Whether `schema` is a release that declares this rule.
    ///
    /// Tables of no recognised release declare none of the registered
    /// rules: a rule is never carried across versions, nor onto an unknown
    /// one.
    #[must_use]
    pub fn applies_to(&self, schema: &Schema) -> bool {
        schema
            .version()
            .is_some_and(|version| self.releases.contains(&version))
    }
}

/// Reason strings, shared so the same gap reads identically everywhere.
const NEEDS_EXPRESSIONS: &str = "requires an EXPRESS expression evaluator";
const NEEDS_BOUNDS: &str = "requires aggregate bounds, which the schema parser does not retain";
const NEEDS_INVERSES: &str =
    "not implemented uniformly: IFC2X3 requires INVERSE relationship semantics, which validation does not derive";
const NEEDS_GEOMETRY: &str = "requires geometric evaluation, which validation does not perform";

/// Every bundled release.
const ALL: &[SchemaVersion] = &[
    SchemaVersion::Ifc2x3,
    SchemaVersion::Ifc4,
    SchemaVersion::Ifc4x3,
];
/// IFC2X3 alone: IFC4 renamed most of its `WRnn` labels.
const IFC2X3: &[SchemaVersion] = &[SchemaVersion::Ifc2x3];
/// IFC4 and IFC4X3, which share their rule labels.
const IFC4_FAMILY: &[SchemaVersion] = &[SchemaVersion::Ifc4, SchemaVersion::Ifc4x3];

/// Every rule this validator knows about, implemented or not.
///
/// Deliberately not exhaustive over every bundled IFC release: claiming to
/// enumerate all rules would be its own dishonesty. It covers selected
/// high-value predicates plus representative unsupported categories.
///
/// Every entity-scoped entry's id, entity and releases are checked against
/// the normative EXPRESS by `tests/registry_scope.rs`.
pub const RULES: &[RuleEntry] = &[
    RuleEntry {
        id: "global.IfcSingleProjectInstance",
        entity: None,
        releases: ALL,
        support: Support::Implemented,
    },
    // IfcRoot constrains GlobalId with `UNIQUE UR1`, not a WHERE rule. It is
    // registered here under its global id rather than as `IfcRoot.UR1`: the
    // check is file-wide, and two entries for one check would double-report.
    RuleEntry {
        id: "global.UniqueGlobalId",
        entity: None,
        releases: ALL,
        support: Support::Implemented,
    },
    RuleEntry {
        id: "IfcRelDefinesByProperties.NoRelatedTypeObject",
        entity: Some("IfcRelDefinesByProperties"),
        releases: IFC4_FAMILY,
        support: Support::Implemented,
    },
    RuleEntry {
        id: "IfcExternalReference.WR1",
        entity: Some("IfcExternalReference"),
        releases: ALL,
        support: Support::Implemented,
    },
    // One predicate under two labels: IFC2X3 `WR1`, IFC4 on
    // `AvoidInconsistentSequence`.
    RuleEntry {
        id: "IfcRelSequence.WR1",
        entity: Some("IfcRelSequence"),
        releases: IFC2X3,
        support: Support::Implemented,
    },
    RuleEntry {
        id: "IfcRelSequence.AvoidInconsistentSequence",
        entity: Some("IfcRelSequence"),
        releases: IFC4_FAMILY,
        support: Support::Implemented,
    },
    // IFC2X3 states these on the abstract IfcRelDecomposes as `WR31`, and
    // the assignment ones below as `WR1`; neither label is registered yet.
    RuleEntry {
        id: "IfcRelAggregates.NoSelfReference",
        entity: Some("IfcRelAggregates"),
        releases: IFC4_FAMILY,
        support: Support::Implemented,
    },
    RuleEntry {
        id: "IfcRelNests.NoSelfReference",
        entity: Some("IfcRelNests"),
        releases: IFC4_FAMILY,
        support: Support::Implemented,
    },
    RuleEntry {
        id: "IfcMaterialLayer.NormalizedPriority",
        entity: Some("IfcMaterialLayer"),
        releases: IFC4_FAMILY,
        support: Support::Implemented,
    },
    RuleEntry {
        id: "IfcRelAssignsToActor.NoSelfReference",
        entity: Some("IfcRelAssignsToActor"),
        releases: IFC4_FAMILY,
        support: Support::Implemented,
    },
    RuleEntry {
        id: "IfcRelAssignsToProcess.NoSelfReference",
        entity: Some("IfcRelAssignsToProcess"),
        releases: IFC4_FAMILY,
        support: Support::Implemented,
    },
    RuleEntry {
        id: "IfcRelAssignsToProduct.NoSelfReference",
        entity: Some("IfcRelAssignsToProduct"),
        releases: IFC4_FAMILY,
        support: Support::Implemented,
    },
    // Declared on IfcRelAssignsToGroup, so IfcRelAssignsToGroupByFactor
    // inherits it rather than declaring its own.
    RuleEntry {
        id: "IfcRelAssignsToGroup.NoSelfReference",
        entity: Some("IfcRelAssignsToGroup"),
        releases: IFC4_FAMILY,
        support: Support::Implemented,
    },
    RuleEntry {
        id: "IfcRelConnectsPathElements.NormalizedRelatingPriorities",
        entity: Some("IfcRelConnectsPathElements"),
        releases: IFC4_FAMILY,
        support: Support::Implemented,
    },
    RuleEntry {
        id: "IfcRelConnectsPathElements.NormalizedRelatedPriorities",
        entity: Some("IfcRelConnectsPathElements"),
        releases: IFC4_FAMILY,
        support: Support::Implemented,
    },
    RuleEntry {
        id: "IfcRelSpaceBoundary.CorrectPhysOrVirt",
        entity: Some("IfcRelSpaceBoundary"),
        releases: IFC4_FAMILY,
        support: Support::Implemented,
    },
    RuleEntry {
        id: "IfcDocumentReference.WR1",
        entity: Some("IfcDocumentReference"),
        releases: ALL,
        support: Support::Unsupported(NEEDS_INVERSES),
    },
    RuleEntry {
        id: "IfcRepresentationContextSameWCS",
        entity: None,
        releases: ALL,
        support: Support::Unsupported(NEEDS_GEOMETRY),
    },
    // One predicate under two labels: IFC2X3 `WR21`, IFC4 on
    // `AllPointsSameDim`.
    RuleEntry {
        id: "IfcPolyLoop.WR21",
        entity: Some("IfcPolyLoop"),
        releases: IFC2X3,
        support: Support::Unsupported(NEEDS_BOUNDS),
    },
    RuleEntry {
        id: "IfcPolyLoop.AllPointsSameDim",
        entity: Some("IfcPolyLoop"),
        releases: IFC4_FAMILY,
        support: Support::Unsupported(NEEDS_BOUNDS),
    },
    RuleEntry {
        id: "IfcQuantityLength.WR21",
        entity: Some("IfcQuantityLength"),
        releases: ALL,
        support: Support::Unsupported(NEEDS_EXPRESSIONS),
    },
    RuleEntry {
        id: "IfcZone.WR1",
        entity: Some("IfcZone"),
        releases: ALL,
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

    /// An entity-scoped id names its declaring entity, and that entity
    /// exists in every release the entry claims.
    ///
    /// `IfcRelAssignsToGroupByFactor.NoSelfReference` once named a subtype
    /// that declares nothing; `tests/registry_scope.rs` additionally checks
    /// each label against the normative EXPRESS.
    #[test]
    fn entity_ids_name_their_declaring_entity_in_every_claimed_release() {
        for entry in RULES {
            assert!(!entry.releases.is_empty(), "{} applies nowhere", entry.id);
            let Some(entity) = entry.entity else {
                continue;
            };
            assert!(
                entry.id.starts_with(&format!("{entity}.")),
                "{} is not `{entity}.<label>`",
                entry.id
            );
            for version in entry.releases {
                let schema = ifc_schema::for_version(*version).expect("bundled tables");
                assert!(
                    schema.entity(entity).is_some(),
                    "{} claims {version:?}, which declares no {entity}",
                    entry.id
                );
            }
        }
    }

    /// Tables of an unrecognised release run no registered rule.
    #[test]
    fn an_unknown_release_declares_no_registered_rule() {
        let schema = Schema::from_express("SCHEMA IFC9;\nEND_SCHEMA;\n");
        assert!(RULES.iter().all(|entry| !entry.applies_to(&schema)));
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
