//! A where-rule violation, named the way the schema names it.

use ifc_model::EntityId;

/// What kind of schema condition was broken.
///
/// Kept separate from the message so a consumer can filter (
/// "show me only degeneracies") without string matching.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViolationKind {
    /// Coordinate-space mismatch, e.g. a 2D direction on a 3D placement.
    Dimensionality,
    /// The values describe something with no extent or no unique solution:
    /// a zero-length direction, an axis parallel to its reference direction.
    Degenerate,
    /// Two things that must agree do not, e.g. boolean operands of
    /// different dimensionality.
    Disagreement,
    /// A value outside the range the schema permits.
    OutOfRange,
    /// An attribute holds an entity type the select does not admit.
    WrongType,
}

/// One broken `WHERE` rule.
///
/// Carries the schema's own rule name (`AxisToRefDirPosition`), not a
/// paraphrase, so a user can look it up in the IFC documentation. That is the
/// difference between a diagnostic someone can act on and one they can only
/// report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleViolation {
    /// The entity that violates the rule.
    pub entity: EntityId,
    /// Its IFC type, e.g. `IFCAXIS2PLACEMENT3D`.
    pub type_name: String,
    /// The rule's name in the EXPRESS schema.
    pub rule: &'static str,
    /// The class of problem.
    pub kind: ViolationKind,
    /// What specifically went wrong.
    pub detail: String,
}

impl RuleViolation {
    /// Build a violation.
    pub fn new(
        entity: EntityId,
        type_name: impl Into<String>,
        rule: &'static str,
        kind: ViolationKind,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            entity,
            type_name: type_name.into(),
            rule,
            kind,
            detail: detail.into(),
        }
    }
}

impl std::fmt::Display for RuleViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {} violates {}: {}",
            self.entity, self.type_name, self.rule, self.detail
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The message must name the schema rule so it can be looked up.
    #[test]
    fn display_names_the_schema_rule_and_the_entity() {
        let v = RuleViolation::new(
            EntityId(4711),
            "IFCAXIS2PLACEMENT3D",
            "AxisToRefDirPosition",
            ViolationKind::Degenerate,
            "Axis is parallel to RefDirection",
        );
        let text = v.to_string();
        assert!(text.contains("#4711"), "{text}");
        assert!(text.contains("AxisToRefDirPosition"), "{text}");
        assert!(text.contains("parallel"), "{text}");
    }
}
