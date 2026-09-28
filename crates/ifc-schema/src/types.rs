//! Defined types, enumerations, and selects used by IFC schemas.

/// Structural shape of a `TYPE` declaration.
///
/// `#[non_exhaustive]`: a match needs a wildcard arm, so recognising a
/// further declaration form later is not a breaking change.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum TypeKind {
    /// Alias or other right-hand-side syntax retained as text.
    Defined(String),
    /// `ENUMERATION OF` member names.
    Enumeration(Vec<String>),
    /// `SELECT` member type names.
    Select(Vec<String>),
}

/// One `TYPE` declaration.
///
/// `#[non_exhaustive]`: build one with [`TypeDef::new`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct TypeDef {
    /// Declared type name.
    pub name: String,
    /// Structurally recognized declaration kind.
    pub kind: TypeKind,
}

impl TypeDef {
    /// Creates a type declaration.
    #[must_use]
    pub fn new(name: impl Into<String>, kind: TypeKind) -> Self {
        Self {
            name: name.into(),
            kind,
        }
    }

    /// Returns whether this declaration aliases another type.
    #[must_use]
    pub const fn is_defined(&self) -> bool {
        matches!(self.kind, TypeKind::Defined(_))
    }
}
