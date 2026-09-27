//! Structured failures for structural analysis projections and authoring.

use ifc_model::EntityId;

/// Result type for `ifc-structural` operations.
pub type StructuralResult<T> = Result<T, StructuralError>;

/// Why a structural projection or draft was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum StructuralError {
    /// The model header declares no `FILE_SCHEMA` token.
    MissingSchema,
    /// The header declares more than one schema, so the profile is ambiguous.
    AmbiguousSchema {
        /// Every schema token found in the header.
        tokens: Vec<String>,
    },
    /// The declared schema token is not IFC2X3, IFC4, or IFC4X3.
    UnsupportedSchema {
        /// The rejected schema token.
        token: String,
    },
    /// An entity id was not found in the model.
    EntityNotFound {
        /// The id that resolved to nothing.
        id: EntityId,
    },
    /// An entity was not the IFC type (or a subtype of it) the caller requested.
    WrongType {
        /// The entity that was read.
        id: EntityId,
        /// The IFC type or supertype required.
        expected: &'static str,
        /// The IFC type actually declared.
        actual: String,
    },
    /// The entity's declared type has no attribute of this name in the selected schema.
    MissingAttribute {
        /// The entity that was read.
        entity: EntityId,
        /// The attribute name that does not exist for this type in this schema version.
        attribute: &'static str,
    },
    /// A staged draft omitted an attribute that is mandatory for its IFC entity type.
    MissingRequired {
        /// The IFC entity type being authored.
        entity_type: String,
        /// The mandatory attribute that was left unset.
        attribute: String,
    },
    /// A staged draft set an attribute that does not exist for its type in the target schema.
    UnsupportedAttribute {
        /// The IFC entity type being authored.
        entity_type: String,
        /// The attribute name absent from this schema version.
        attribute: String,
    },
    /// An attribute held a value of the wrong kind or shape for the schema.
    InvalidValue {
        /// The entity that was read.
        entity: EntityId,
        /// The attribute holding the invalid value.
        attribute: &'static str,
        /// The kind of value that was expected instead.
        expected: &'static str,
    },
    /// A staged draft supplied a value of the wrong kind or shape for the attribute.
    InvalidDraftValue {
        /// The IFC entity type being authored.
        entity_type: &'static str,
        /// The attribute holding the invalid value.
        attribute: &'static str,
        /// The kind of value that was expected instead.
        expected: &'static str,
    },
    /// An entity reference attribute points at an id absent from the model.
    DanglingReference {
        /// The entity holding the dangling reference.
        entity: EntityId,
        /// The attribute holding the dangling reference.
        attribute: &'static str,
        /// The id that resolved to nothing.
        target: EntityId,
    },
    /// An entity reference resolved, but the target is not one of the types the slot allows.
    WrongReferenceType {
        /// The entity holding the reference.
        entity: EntityId,
        /// The attribute holding the reference.
        attribute: &'static str,
        /// The entity that was read.
        target: EntityId,
        /// The IFC type or select members required.
        expected: &'static str,
        /// The IFC type actually declared by the target.
        actual: String,
    },
    /// A SET/LIST attribute had fewer or more members than the schema allows.
    InvalidCardinality {
        /// The entity holding the aggregate attribute.
        entity: EntityId,
        /// The attribute whose aggregate size is out of bounds.
        attribute: &'static str,
        /// Minimum allowed member count.
        minimum: usize,
        /// Maximum allowed member count, or `None` if unbounded.
        maximum: Option<usize>,
        /// The member count actually found.
        actual: usize,
    },
    /// A structural semantic rule beyond plain schema shape was violated.
    ///
    /// Used for constraints the EXPRESS schema cannot encode directly, such
    /// as forbidding an `IfcRelAssignsToGroup` from assigning its own
    /// `RelatingGroup`, or requiring `ObjectType` when `PredefinedType` is
    /// `USERDEFINED`.
    SemanticViolation {
        /// The entity that violated the rule, if the violation is entity-scoped.
        entity: Option<EntityId>,
        /// A short identifier naming the violated rule.
        rule: &'static str,
    },
    /// `GlobalId` was not a well-formed 22-character IFC GUID.
    InvalidGlobalId,
}

impl std::fmt::Display for StructuralError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingSchema => f.write_str("the model header declares no IFC schema"),
            Self::AmbiguousSchema { tokens } => write!(f, "the model declares multiple schemas: {tokens:?}"),
            Self::UnsupportedSchema { token } => write!(f, "unsupported IFC schema `{token}`"),
            Self::EntityNotFound { id } => write!(f, "entity {id} does not exist"),
            Self::WrongType { id, expected, actual } => write!(f, "entity {id} is {actual}, expected {expected}"),
            Self::MissingAttribute { entity, attribute } => write!(f, "entity {entity} has no `{attribute}` attribute in the selected schema"),
            Self::MissingRequired { entity_type, attribute } => write!(f, "{entity_type}.{attribute} is required"),
            Self::UnsupportedAttribute { entity_type, attribute } => write!(f, "{entity_type}.{attribute} does not exist in the selected schema"),
            Self::InvalidValue { entity, attribute, expected } => write!(f, "entity {entity} has invalid `{attribute}`; expected {expected}"),
            Self::InvalidDraftValue { entity_type, attribute, expected } => write!(f, "draft {entity_type} has invalid `{attribute}`; expected {expected}"),
            Self::DanglingReference { entity, attribute, target } => write!(f, "entity {entity}.{attribute} refers to missing {target}"),
            Self::WrongReferenceType { entity, attribute, target, expected, actual } => write!(f, "entity {entity}.{attribute} refers to {target} ({actual}), expected {expected}"),
            Self::InvalidCardinality { entity, attribute, minimum, maximum, actual } => write!(f, "entity {entity}.{attribute} cardinality {actual} is outside {minimum}..{maximum:?}"),
            Self::SemanticViolation { rule, .. } => write!(f, "structural semantic rule `{rule}` failed"),
            Self::InvalidGlobalId => f.write_str("GlobalId is not a valid 22-character IFC GUID"),
        }
    }
}

impl std::error::Error for StructuralError {}
