//! One positional attribute slot on an IFC entity.
//!
//! This crate owns the type rather than re-exporting the EXPRESS extractor's:
//! the bundled tables are decoded straight into it, so a release of the
//! parser never changes this crate's public API.

/// One explicit positional attribute declared by an entity.
///
/// `#[non_exhaustive]`: build one with [`Attribute::new`] and the builder
/// methods. Further facts about a declaration (aggregate bounds, for
/// example) are added as new fields without breaking existing readers.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Attribute {
    /// Declared attribute name.
    pub name: String,
    /// Declared scalar or element type token, e.g. `IfcLabel`.
    pub type_name: String,
    /// Whether `OPTIONAL` was present.
    pub optional: bool,
    /// Whether a `LIST`, `SET`, `ARRAY`, or `BAG` wrapper was present.
    pub aggregate: bool,
}

impl Attribute {
    /// Creates a required scalar attribute.
    #[must_use]
    pub fn new(name: impl Into<String>, type_name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            type_name: type_name.into(),
            optional: false,
            aggregate: false,
        }
    }

    /// Marks the attribute as optional.
    #[must_use]
    pub const fn optional(mut self) -> Self {
        self.optional = true;
        self
    }

    /// Marks the attribute as an aggregate.
    #[must_use]
    pub const fn aggregate(mut self) -> Self {
        self.aggregate = true;
        self
    }
}
