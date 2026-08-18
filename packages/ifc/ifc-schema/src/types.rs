//! Defined types, enumerations, and selects.
//!
//! IFC4 declares 397 of these alongside its 776 entities. They matter for
//! reading because a STEP value may be wrapped in its type name
//! (`IFCLENGTHMEASURE(0.2)`), and for ifcXML because the wrapper becomes an
//! element name.

/// What an EXPRESS `TYPE` declaration actually declares.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeKind {
    /// An alias for a base type, e.g. `IfcLengthMeasure = REAL`.
    Defined(String),
    /// A closed set of names, e.g. `IfcWallTypeEnum`.
    Enumeration(Vec<String>),
    /// A union of other types, e.g. `IfcValue`.
    Select(Vec<String>),
}

/// One `TYPE` declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeDef {
    /// Name as declared, e.g. `IfcLengthMeasure`.
    pub name: String,
    /// What it declares.
    pub kind: TypeKind,
}

impl TypeDef {
    /// Is this a measure-like alias whose STEP wrapper carries a number?
    pub fn is_defined(&self) -> bool {
        matches!(self.kind, TypeKind::Defined(_))
    }
}
