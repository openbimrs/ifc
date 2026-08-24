//! The applicable/failed/not-applicable trichotomy.

/// The result of checking one requirement against one element.
///
/// # Why three states and not a `bool`
///
/// A check that reports "data missing" as "passed" is worse than no check: it
/// launders absence into compliance. Every openBIM validation surface
/// (`openbim-ids` audits, `openbim-loin` conformance) must therefore be able
/// to say *"this did not apply"* distinctly from *"this passed"*.
///
/// The sibling `../vendor/solibri` engine makes the same distinction explicit
/// in its rule layer; its notes record what breaks when it does not.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Outcome {
    /// The requirement applied to this element, and the element satisfied it.
    Passed,
    /// The requirement applied to this element, and the element did not.
    Failed,
    /// The requirement did not apply — the element was out of scope.
    ///
    /// Distinct from [`Outcome::Passed`]. Collapsing the two is the failure
    /// mode this enum exists to prevent.
    NotApplicable,
}

impl Outcome {
    /// Whether the requirement was in scope at all.
    ///
    /// ```
    /// use openbim_core::Outcome;
    /// assert!(Outcome::Failed.is_applicable());
    /// assert!(!Outcome::NotApplicable.is_applicable());
    /// ```
    #[must_use]
    pub fn is_applicable(self) -> bool {
        !matches!(self, Outcome::NotApplicable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn not_applicable_is_not_a_pass() {
        assert!(Outcome::Passed.is_applicable());
        assert!(Outcome::Failed.is_applicable());
        assert!(!Outcome::NotApplicable.is_applicable());
        assert_ne!(Outcome::Passed, Outcome::NotApplicable);
    }
}
