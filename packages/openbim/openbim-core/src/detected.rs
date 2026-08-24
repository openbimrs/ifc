//! Version detection that reports its own evidence.

/// How a document's version was determined.
///
/// # The trap this closes
///
/// Several openBIM schemas reuse one XML namespace across incompatible
/// versions. IDS is the extreme case: every published version from 0.2 to
/// 1.0 declares a byte-identical `targetNamespace`, and the differences are in
/// attribute *names* and cardinality rather than element names. A reader that
/// guesses wrong therefore does not fail — it silently yields a *different*
/// specification.
///
/// So detection returns its evidence, and a file whose declared version
/// disagrees with its actual shape produces [`Detected::Conflict`] rather than
/// a silent pick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Detected<V> {
    /// The document stated its version and the content agreed.
    Declared(V),
    /// The version was inferred from document shape; nothing declared it.
    Inferred(V),
    /// The declared version and the observed shape disagree.
    ///
    /// Never resolve this silently. Which one is correct is a policy decision
    /// belonging to the caller, not the parser.
    Conflict {
        /// What the document claimed.
        declared: V,
        /// What its shape indicates.
        observed: V,
    },
}

impl<V: Copy> Detected<V> {
    /// The version to use when the caller has no opinion about conflicts.
    ///
    /// Returns `None` for [`Detected::Conflict`] — resolving that is the
    /// caller's decision, and defaulting it here would reintroduce exactly the
    /// silent-wrong-parse failure this type exists to surface.
    ///
    /// ```
    /// use openbim_core::Detected;
    /// assert_eq!(Detected::Declared(1u8).resolved(), Some(1));
    /// assert_eq!(
    ///     Detected::Conflict { declared: 1u8, observed: 2 }.resolved(),
    ///     None
    /// );
    /// ```
    #[must_use]
    pub fn resolved(&self) -> Option<V> {
        match self {
            Detected::Declared(v) | Detected::Inferred(v) => Some(*v),
            Detected::Conflict { .. } => None,
        }
    }

    /// Whether detection found contradictory evidence.
    #[must_use]
    pub fn is_conflict(&self) -> bool {
        matches!(self, Detected::Conflict { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conflict_refuses_to_resolve_itself() {
        assert_eq!(Detected::Declared(3u8).resolved(), Some(3));
        assert_eq!(Detected::Inferred(3u8).resolved(), Some(3));

        let conflict = Detected::Conflict {
            declared: 2u8,
            observed: 3,
        };
        assert_eq!(conflict.resolved(), None);
        assert!(conflict.is_conflict());
    }

    #[test]
    fn agreement_is_not_a_conflict() {
        assert!(!Detected::Declared(1u8).is_conflict());
        assert!(!Detected::Inferred(1u8).is_conflict());
    }
}
