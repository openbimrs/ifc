//! Attribute descriptors and their declared types.

/// One positional attribute slot on an entity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attribute {
    /// Attribute name as declared in EXPRESS (e.g. `GlobalId`).
    pub name: String,
    /// Whether the schema marks it `OPTIONAL` (may legitimately be `$`).
    pub optional: bool,
    /// Whether it is `DERIVE`d in this entity (appears as `*` in the record).
    pub derived: bool,
    /// The declared type, used to interpret the raw STEP value.
    pub kind: AttributeKind,
}

/// The declared type of an attribute, at the granularity a reader needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttributeKind {
    /// A simple value: integer, real, string, boolean, logical.
    Simple,
    /// A reference to another entity instance.
    Reference,
    /// An aggregate (`SET`, `LIST`, `ARRAY`, `BAG`) of the inner kind.
    Aggregate(Box<AttributeKind>),
    /// A named `TYPE`, `ENUMERATION` or `SELECT` declared in the schema.
    Named(String),
}
