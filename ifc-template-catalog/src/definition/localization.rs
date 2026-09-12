//! Localized publication text.

/// A language-tagged alias preserved from the source catalog.
#[derive(Debug, Clone, PartialEq, Eq, bincode::Encode, bincode::Decode)]
pub struct LocalizedText {
    /// IETF-style language tag as declared by the publication, absent when unspecified.
    pub language: Option<String>,
    /// The alias text itself.
    pub text: String,
}
