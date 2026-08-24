//! `openbim-bcf` — BIM Collaboration Format.
//!
//! # What this is
//!
//! The open issue-exchange format: a ZIP with one directory per topic, each
//! holding the issue XML and optionally a viewpoint (camera plus component
//! visibility) and a snapshot image. It is how findings from an audit leave
//! this toolchain and land in any BCF-aware reviewer.
//!
//! # BCF is two standards
//!
//! **BCF-XML** is the file container this crate targets. **BCF-API** is a
//! separate REST/JSON service specification for the same domain. They share a
//! data model and nothing else; conflating them is why this crate is not
//! simply named for the file extension.
//!
//! # 🚨 The reader must be tolerant, and that is evidence-based
//!
//! Measured over 33 real third-party archives in the sibling
//! `../vendor/solibri` corpus:
//!
//! | Spec says | Corpus says |
//! | --- | --- |
//! | `project.bcfp` describes the project | **0 of 33** have one |
//! | `bcf.version` declares the version | **20 of 33** have none |
//! | `TopicStatus` comes from a known set | free text, e.g. `"Offen"` |
//!
//! A spec-strict reader rejects every file in that corpus — files every other
//! BIM tool opens without complaint. So: reject only what cannot be
//! interpreted at all, and keep status/type strings **verbatim** rather than
//! mapping them onto an enum. `"Offen"` is not a parse failure; it is what the
//! file says, and normalising it would corrupt a round-trip.
//!
//! # Status
//!
//! **Reserved — no implementation.** Published to establish the name.
//! Read and write support are tracked separately and must never be inferred
//! from one another.

#![forbid(unsafe_code)]

/// A BCF-XML container version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BcfVersion {
    /// 2.0 — comment status in child elements; a back-reference `Topic`
    /// element is nested inside each comment.
    V2_0,
    /// 2.1 — status moves onto a `Topic` attribute; the nested back-reference
    /// is dropped.
    V2_1,
    /// 3.0.
    V3_0,
}

impl BcfVersion {
    /// Whether documents of this version nest a back-reference `Topic` element
    /// inside each comment.
    ///
    /// A reader that does not expect this will mistake the back-reference for
    /// a second topic declaration.
    #[must_use]
    pub fn nests_topic_in_comment(self) -> bool {
        matches!(self, BcfVersion::V2_0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_2_0_nests_topic_backreferences() {
        assert!(BcfVersion::V2_0.nests_topic_in_comment());
        assert!(!BcfVersion::V2_1.nests_topic_in_comment());
        assert!(!BcfVersion::V3_0.nests_topic_in_comment());
    }

    #[test]
    fn versions_order_oldest_first() {
        assert!(BcfVersion::V2_0 < BcfVersion::V2_1);
        assert!(BcfVersion::V2_1 < BcfVersion::V3_0);
    }
}
