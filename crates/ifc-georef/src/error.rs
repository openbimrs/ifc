//! Structured failures while resolving an IFC coordinate operation.

use ifc_model::EntityId;

/// Why a project-to-map operation could not be resolved losslessly.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub enum GeorefError {
    /// An entity referenced another that the model does not contain.
    MissingEntity {
        /// The entity holding the dangling reference.
        referrer: EntityId,
        /// The id that resolved to nothing.
        missing: EntityId,
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
    /// A value supplied to an authoring helper was not acceptable.
    ///
    /// Distinct from [`GeorefError::InvalidAttribute`], which names an
    /// entity already in the model: at authoring time there is no id
    /// yet, so the entity is named by type.
    AuthoringInvalid {
        /// The entity being authored.
        entity: &'static str,
        /// The attribute at fault.
        attribute: &'static str,
        /// What was supplied.
        value: String,
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
    /// The coordinate operation is a subtype this crate does not resolve.
    ///
    /// Refused rather than approximated: a non-projected operation would
    /// silently produce wrong map coordinates.
    UnsupportedOperation {
        /// The coordinate operation entity.
        entity: EntityId,
        /// The IFC type actually declared.
        actual: String,
    },
    /// A rigid operation's coordinates are the other `SameCoordinateType`
    /// branch than the entry point reads: plane angles given to the
    /// project-to-map resolver, which only lowers length offsets, or
    /// lengths given to the geographic-offset reader.
    CoordinateMeasureMismatch {
        /// The `IfcRigidOperation` entity.
        entity: EntityId,
        /// The measure type this entry point reads.
        expected: &'static str,
        /// The measure type both coordinates carry.
        actual: &'static str,
    },
    /// The map x axis is zero-length or non-finite, so no rotation exists.
    DegenerateAxis {
        /// The map conversion entity.
        entity: EntityId,
    },
    /// The map scale is zero, negative, or non-finite.
    InvalidScale {
        /// The map conversion entity.
        entity: EntityId,
        /// The rejected scale.
        value: f64,
    },
    /// The map unit is not a length unit this crate can reduce to metres.
    InvalidUnit {
        /// The unit entity.
        entity: EntityId,
        /// Which expectation failed.
        detail: &'static str,
    },
    /// A unit conversion chain revisited an entity or ran deeper than 16.
    ///
    /// Both guards exist: a repeated id catches a true loop, the depth cap
    /// catches an unbounded chain of distinct units.
    UnitCycle {
        /// The unit entity where the walk was abandoned.
        entity: EntityId,
    },
    /// The model header declares no `FILE_SCHEMA` token.
    MissingSchema,
    /// The header declares more than one schema, so the profile is ambiguous.
    ///
    /// Refused rather than picking one: the choice changes which coordinate
    /// operations are legal.
    AmbiguousSchema {
        /// Every schema token found in the header.
        tokens: Vec<String>,
    },
    /// The declared schema has no projected georeferencing profile here.
    ///
    /// IFC2X3 declares no georeferencing entities at all; other tokens may be
    /// unrecognised.
    UnsupportedSchema {
        /// The rejected schema token.
        token: String,
    },
    /// Chaining onto a project frame requires that frame to be supplied.
    MissingProjectFrame,
    /// The supplied project frame is not an invertible rigid transform.
    DegenerateProjectFrame,
    /// A direction was zero-length or non-finite, so it cannot be normalised.
    NonFiniteDirection {
        /// The direction entity.
        entity: EntityId,
    },
    /// An entity breaks a WHERE rule or an inverse cardinality its declared
    /// release states, such as IFC4X3 `IfcCoordinateReferenceSystem.NameOrWKT`.
    RuleViolation {
        /// The entity that breaks the rule.
        entity: EntityId,
        /// The rule's schema label, such as `NameOrWKT`.
        rule: &'static str,
    },
    /// An `IfcCompoundPlaneAngleMeasure` breaks a WHERE rule of its type in
    /// the declared release (such as IFC4 `ConsistentSign` or IFC2X3 `WR1`),
    /// or the WGS84 range its attribute definition states.
    InvalidCompoundAngle {
        /// The entity holding the angle.
        entity: EntityId,
        /// Zero-based slot index of the angle attribute.
        index: usize,
        /// Schema name of the attribute, such as `RefLatitude`.
        name: &'static str,
        /// The rule's schema label, or the stated range that is broken.
        rule: &'static str,
    },
    /// A caller-supplied number is non-finite or outside its documented
    /// domain, such as a negative comparison tolerance.
    InvalidParameter {
        /// The parameter's name.
        name: &'static str,
        /// The rejected value.
        value: f64,
    },
}

impl std::fmt::Display for GeorefError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingEntity { referrer, missing } => {
                write!(f, "{referrer} references missing {missing}")
            }
            Self::WrongType {
                entity,
                expected,
                actual,
            } => write!(f, "{entity} is {actual}, expected {expected}"),
            Self::MissingAttribute {
                entity,
                index,
                name,
            } => write!(f, "{entity} is missing {name} at slot {index}"),
            Self::InvalidAttribute {
                entity,
                index,
                name,
            } => write!(f, "{entity} has invalid {name} at slot {index}"),
            Self::AuthoringInvalid {
                entity,
                attribute,
                value,
            } => write!(f, "{entity}.{attribute}: {value}"),
            Self::UnsupportedOperation { entity, actual } => {
                write!(f, "{entity} uses unsupported coordinate operation {actual}")
            }
            Self::CoordinateMeasureMismatch {
                entity,
                expected,
                actual,
            } => write!(
                f,
                "{entity} has {actual} coordinates, this reader requires {expected}"
            ),
            Self::DegenerateAxis { entity } => {
                write!(f, "{entity} has a zero-length or non-finite map x axis")
            }
            Self::InvalidScale { entity, value } => {
                write!(f, "{entity} has invalid map scale {value}")
            }
            Self::InvalidUnit { entity, detail } => {
                write!(f, "{entity} has unsupported or invalid map unit: {detail}")
            }
            Self::UnitCycle { entity } => {
                write!(f, "unit conversion chain at {entity} is cyclic or too deep")
            }
            Self::MissingSchema => {
                write!(f, "model header declares no FILE_SCHEMA token")
            }
            Self::AmbiguousSchema { tokens } => {
                write!(f, "model header declares multiple schemas: {tokens:?}")
            }
            Self::UnsupportedSchema { token } => {
                write!(
                    f,
                    "schema {token} does not declare IFC georeferencing entities \
                     or its coordinate-operation profile is not projected here"
                )
            }
            Self::MissingProjectFrame => {
                write!(
                    f,
                    "chaining a project-to-map operation onto a project frame \
                     requires that frame to be supplied explicitly"
                )
            }
            Self::DegenerateProjectFrame => {
                write!(
                    f,
                    "supplied project frame is not an invertible rigid transform \
                     (zero or non-finite determinant)"
                )
            }
            Self::NonFiniteDirection { entity } => {
                write!(f, "{entity} has a non-finite or zero-length direction")
            }
            Self::RuleViolation { entity, rule } => {
                write!(f, "{entity} violates {rule}")
            }
            Self::InvalidCompoundAngle {
                entity,
                index,
                name,
                rule,
            } => write!(f, "{entity} {name} at slot {index} violates {rule}"),
            Self::InvalidParameter { name, value } => {
                write!(f, "parameter {name} has invalid value {value}")
            }
        }
    }
}

impl std::error::Error for GeorefError {}

/// Result of a georeferencing read, carrying [`GeorefError`] on failure.
pub type GeorefResult<T> = Result<T, GeorefError>;
