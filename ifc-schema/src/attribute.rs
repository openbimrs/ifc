//! One positional attribute slot on an entity.

/// An explicit attribute declared by an entity.
///
/// Only *explicit* attributes are recorded. `DERIVE` attributes do not occupy
/// a positional slot in a STEP record, and `INVERSE` attributes are
/// back-references rather than stored data, so including either would shift
/// every subsequent index and corrupt attribute lookup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attribute {
    /// Attribute name as declared in EXPRESS, e.g. `Coordinates`.
    ///
    /// ifcXML writes this as the XML attribute or child element name, which is
    /// why the schema is required for a conformant XML codec.
    pub name: String,
    /// The declared type token, e.g. `IfcLengthMeasure`.
    pub type_name: String,
    /// Whether the slot may be `$` (unset).
    pub optional: bool,
    /// Whether the slot holds an aggregate (`LIST`, `SET`, `ARRAY`, `BAG`).
    pub aggregate: bool,
}

impl Attribute {
    /// A required, scalar attribute.
    pub fn new(name: impl Into<String>, type_name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            type_name: type_name.into(),
            optional: false,
            aggregate: false,
        }
    }

    /// Mark this slot optional.
    pub fn optional(mut self) -> Self {
        self.optional = true;
        self
    }

    /// Mark this slot an aggregate.
    pub fn aggregate(mut self) -> Self {
        self.aggregate = true;
        self
    }
}
