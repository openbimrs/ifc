//! Structured alignment interpretation failures.

use ifc_model::EntityId;

/// Why an IFC alignment entity could not be read or lowered.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub enum AlignmentError {
    /// A referenced entity id is not present in the model.
    MissingEntity {
        /// The id that resolved to nothing.
        entity: EntityId,
    },
    /// An entity was not the IFC type the referencing slot requires.
    WrongType {
        /// The entity that was read.
        entity: EntityId,
        /// The IFC type the slot requires.
        expected: &'static str,
        /// The IFC type actually declared.
        actual: String,
    },
    /// A mandatory attribute was absent, so the value cannot be inferred.
    MissingAttribute {
        /// The entity that was read.
        entity: EntityId,
        /// Zero-based slot index of the absent attribute.
        index: usize,
        /// Schema name of the attribute.
        name: &'static str,
    },
    /// An attribute was present but held the wrong kind of value.
    InvalidAttribute {
        /// The entity that was read.
        entity: EntityId,
        /// Zero-based slot index of the offending attribute.
        index: usize,
        /// Schema name of the attribute.
        name: &'static str,
    },
    /// The project's unit assignment does not resolve to usable lengths or angles.
    InvalidUnits {
        /// What expectation failed.
        detail: &'static str,
    },
    /// A horizontal, vertical, or cant segment carries internally inconsistent data.
    InvalidSegment {
        /// The segment entity.
        entity: EntityId,
        /// What expectation failed.
        detail: &'static str,
    },
    /// A segment type or configuration this crate deliberately does not lower.
    ///
    /// Covers segment kinds that are legal IFC but require data this crate does
    /// not yet combine (e.g. `VIENNESEBEND` needs the cant swing carried by a
    /// separate `IfcAlignmentCant` layout) or that have no closed-form neutral
    /// curve here (e.g. `CUBIC`).
    Unsupported {
        /// The unsupported entity.
        entity: EntityId,
        /// The IFC type or `PredefinedType` name actually declared.
        type_name: String,
        /// Why it is refused rather than approximated.
        detail: &'static str,
    },
    /// The assembled neutral alignment graph violates a structural invariant.
    Graph {
        /// What expectation failed.
        detail: String,
    },
    /// The model header declares no `FILE_SCHEMA` token.
    MissingSchema,
    /// More than one `FILE_SCHEMA` token was declared.
    AmbiguousSchema {
        /// Every schema token found in the header.
        tokens: Vec<String>,
    },
    /// The declared schema is not one this crate can interpret.
    ///
    /// Alignment entities (`IfcAlignment*`) were introduced in IFC4X3; IFC2X3
    /// and IFC4 ADD2 TC1 do not declare them at all, so there is no version
    /// dispatch here the way `ifc-resource`/`ifc-structural` have one --
    /// exactly one profile is authoritative and anything else is refused.
    UnsupportedSchema {
        /// The rejected schema token.
        token: String,
    },
    /// A relationship or nesting structure violates a stated invariant.
    SemanticViolation {
        /// The offending entity, when one could be identified.
        entity: Option<EntityId>,
        /// The invariant that was violated.
        rule: &'static str,
    },
    /// A dangling reference: the target id is not present in the model.
    DanglingReference {
        /// The entity holding the dangling reference.
        entity: EntityId,
        /// The attribute that names the missing target.
        attribute: &'static str,
        /// The id that resolved to nothing.
        target: EntityId,
    },
    /// Graph traversal exceeded an explicit depth or node-count bound.
    BudgetExceeded {
        /// The configured maximum traversal depth.
        max_depth: usize,
        /// The configured maximum node count.
        max_nodes: usize,
    },
}

/// Result of an alignment read, carrying [`AlignmentError`] on failure.
pub type AlignmentResult<T> = Result<T, AlignmentError>;

impl std::fmt::Display for AlignmentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingEntity { entity } => write!(f, "missing alignment entity {entity}"),
            Self::WrongType {
                entity,
                expected,
                actual,
            } => write!(f, "{entity} is {actual}; expected {expected}"),
            Self::MissingAttribute {
                entity,
                index,
                name,
            } => write!(f, "{entity} misses {name} at slot {index}"),
            Self::InvalidAttribute {
                entity,
                index,
                name,
            } => write!(f, "{entity} has invalid {name} at slot {index}"),
            Self::InvalidUnits { detail } => write!(f, "invalid alignment units: {detail}"),
            Self::InvalidSegment { entity, detail } => {
                write!(f, "invalid alignment segment {entity}: {detail}")
            }
            Self::Unsupported {
                entity,
                type_name,
                detail,
            } => write!(f, "unsupported {type_name} at {entity}: {detail}"),
            Self::Graph { detail } => write!(f, "invalid neutral alignment graph: {detail}"),
            Self::MissingSchema => write!(f, "no FILE_SCHEMA token was declared"),
            Self::AmbiguousSchema { tokens } => {
                write!(f, "ambiguous FILE_SCHEMA tokens: {tokens:?}")
            }
            Self::UnsupportedSchema { token } => write!(
                f,
                "unsupported schema {token}: alignment entities require IFC4X3 ADD2"
            ),
            Self::SemanticViolation { entity, rule } => match entity {
                Some(entity) => write!(f, "{entity} violates {rule}"),
                None => write!(f, "violates {rule}"),
            },
            Self::DanglingReference {
                entity,
                attribute,
                target,
            } => write!(f, "{entity}.{attribute} references missing {target}"),
            Self::BudgetExceeded {
                max_depth,
                max_nodes,
            } => write!(
                f,
                "traversal exceeded bounds (max_depth={max_depth}, max_nodes={max_nodes})"
            ),
        }
    }
}

impl std::error::Error for AlignmentError {}
