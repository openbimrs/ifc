//! Localized publication text.

/// A language-tagged alias preserved from the source catalog.
#[derive(Debug, Clone, PartialEq, Eq, bincode::Encode, bincode::Decode)]
pub struct LocalizedText {
    pub language: Option<String>,
    pub text: String,
}
