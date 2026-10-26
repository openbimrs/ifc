//! The members a catalog template declares, in neutral terms (#537).
//!
//! `ifc-validate` links no catalog (ADR 0017), so the facade copies what it
//! needs from a catalog's set templates into these types. Names are the
//! catalog's spelling; value types are IFC defined-type names such as
//! `IfcBoolean`.

/// One catalog set template: what kind of set it is and which members it
/// declares.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct CatalogSet {
    /// Whether the set holds properties or quantities.
    pub kind: CatalogSetKind,
    /// The declared members, in catalog order.
    pub members: Vec<CatalogMember>,
}

impl CatalogSet {
    /// A property set declaring `members`.
    #[must_use]
    pub fn property(members: Vec<CatalogMember>) -> Self {
        Self {
            kind: CatalogSetKind::Property,
            members,
        }
    }

    /// A quantity set declaring `members`.
    #[must_use]
    pub fn quantity(members: Vec<CatalogMember>) -> Self {
        Self {
            kind: CatalogSetKind::Quantity,
            members,
        }
    }
}

/// Whether a catalog set is an `IfcPropertySet` or an `IfcElementQuantity`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum CatalogSetKind {
    /// An `IfcPropertySet`.
    Property,
    /// An `IfcElementQuantity`.
    Quantity,
}

/// One declared property or quantity.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct CatalogMember {
    /// The member's `Name`.
    pub name: String,
    /// The entity form and value types it declares.
    pub form: CatalogMemberForm,
}

impl CatalogMember {
    /// A member named `name` of `form`.
    #[must_use]
    pub fn new(name: impl Into<String>, form: CatalogMemberForm) -> Self {
        Self {
            name: name.into(),
            form,
        }
    }
}

/// The entity form of a declared member, with the value types it declares.
///
/// A `None` value type means the catalog leaves it open: not checked.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CatalogMemberForm {
    /// An `IfcPropertySingleValue`.
    SingleValue {
        /// The `NominalValue` type.
        data_type: Option<String>,
    },
    /// An `IfcPropertyEnumeratedValue`.
    EnumeratedValue {
        /// The type of the `EnumerationValues` members.
        data_type: Option<String>,
    },
    /// An `IfcPropertyBoundedValue`.
    BoundedValue {
        /// The type of the bound and set-point values.
        data_type: Option<String>,
    },
    /// An `IfcPropertyListValue`.
    ListValue {
        /// The type of the `ListValues` members.
        data_type: Option<String>,
    },
    /// An `IfcPropertyTableValue`.
    TableValue {
        /// The type of the `DefiningValues` members.
        defining_type: Option<String>,
        /// The type of the `DefinedValues` members.
        defined_type: Option<String>,
    },
    /// An `IfcPropertyReferenceValue`.
    ReferenceValue {
        /// The entity type the reference must resolve to.
        reference_type: String,
    },
    /// An `IfcComplexProperty` grouping further properties.
    Complex {
        /// The declared nested members.
        members: Vec<CatalogMember>,
    },
    /// A simple quantity of the given kind.
    Quantity(CatalogQuantityKind),
}

impl CatalogMemberForm {
    /// The IFC entity this form is written as.
    #[must_use]
    pub fn entity_name(&self) -> &'static str {
        match self {
            Self::SingleValue { .. } => "IfcPropertySingleValue",
            Self::EnumeratedValue { .. } => "IfcPropertyEnumeratedValue",
            Self::BoundedValue { .. } => "IfcPropertyBoundedValue",
            Self::ListValue { .. } => "IfcPropertyListValue",
            Self::TableValue { .. } => "IfcPropertyTableValue",
            Self::ReferenceValue { .. } => "IfcPropertyReferenceValue",
            Self::Complex { .. } => "IfcComplexProperty",
            Self::Quantity(kind) => kind.entity_name(),
        }
    }
}

/// The family of a simple quantity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum CatalogQuantityKind {
    /// `IfcQuantityLength`.
    Length,
    /// `IfcQuantityArea`.
    Area,
    /// `IfcQuantityVolume`.
    Volume,
    /// `IfcQuantityWeight`.
    Weight,
    /// `IfcQuantityTime`.
    Time,
    /// `IfcQuantityCount`.
    Count,
    /// `IfcQuantityNumber`.
    Number,
}

impl CatalogQuantityKind {
    /// The IFC entity of this kind.
    #[must_use]
    pub fn entity_name(self) -> &'static str {
        match self {
            Self::Length => "IfcQuantityLength",
            Self::Area => "IfcQuantityArea",
            Self::Volume => "IfcQuantityVolume",
            Self::Weight => "IfcQuantityWeight",
            Self::Time => "IfcQuantityTime",
            Self::Count => "IfcQuantityCount",
            Self::Number => "IfcQuantityNumber",
        }
    }
}
