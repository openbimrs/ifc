//! Defined types, enumerations, and selects.
//!
//! IFC4 declares 397 of these alongside its 776 entities. They matter for
//! reading because a STEP value may be wrapped in its type name
//! (`IFCLENGTHMEASURE(3.2)`), and for validation because a select restricts
//! which entities may legally appear in a slot.

/// A non-entity type declaration from the schema.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeDef {
    /// `TYPE X = REAL;` — a simple type alias, possibly with constraints.
    Defined {
        /// The declared name.
        name: String,
        /// The underlying primitive, as written.
        underlying: String,
    },
    /// `TYPE X = ENUMERATION OF (...);`
    Enumeration {
        /// The declared name.
        name: String,
        /// Permitted values, uppercase as written.
        values: Vec<String>,
    },
    /// `TYPE X = SELECT (...);`
    Select {
        /// The declared name.
        name: String,
        /// Permitted member type names.
        members: Vec<String>,
    },
}
