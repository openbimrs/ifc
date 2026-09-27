//! Why interpreting a geometry entity failed.
//!
//! Every failure names the entity that caused it. A geometry bug in a
//! 500k-entity file is unfindable otherwise, and "returned None" tells you
//! nothing about which of 3,000 walls was malformed.

use ifc_model::EntityId;
use thiserror::Error;

/// The result of interpreting IFC geometry.
pub type GeometryResult<T> = Result<T, GeometryError>;

/// Failures when reading or lowering IFC geometry.
#[derive(Debug, Clone, PartialEq, Error)]
#[non_exhaustive]
pub enum GeometryError {
    /// An entity referenced by an attribute is not in the model.
    #[error("{referrer} references missing entity {missing}")]
    MissingEntity {
        /// The entity holding the dangling reference.
        referrer: EntityId,
        /// The id that does not resolve.
        missing: EntityId,
    },

    /// An attribute slot was empty but the geometry needs it.
    #[error("{entity} ({type_name}) has no {attribute}")]
    MissingAttribute {
        /// The offending entity.
        entity: EntityId,
        /// Its IFC type.
        type_name: String,
        /// Which attribute was required.
        attribute: &'static str,
    },

    /// An attribute held a value of the wrong shape.
    #[error("{entity} ({type_name}).{attribute}: expected {expected}, found {found}")]
    WrongValueKind {
        /// The offending entity.
        entity: EntityId,
        /// Its IFC type.
        type_name: String,
        /// Which attribute.
        attribute: &'static str,
        /// What the schema requires.
        expected: &'static str,
        /// What the file actually contains.
        found: String,
    },

    /// The entity is not the type the caller assumed.
    #[error("{entity} is {actual}, not a {expected}")]
    WrongEntityType {
        /// The offending entity.
        entity: EntityId,
        /// The type it actually has.
        actual: String,
        /// The type family that was required.
        expected: &'static str,
    },

    /// A recognized IFC entity whose interpretation is not implemented.
    ///
    /// Distinct from [`Self::WrongEntityType`]: the file is valid and we simply
    /// do not handle it yet. Never silently substituted with a wrong shape.
    #[error("{type_name} ({entity}) is valid IFC but not yet interpreted: {detail}")]
    Unsupported {
        /// The entity in question.
        entity: EntityId,
        /// Its IFC type.
        type_name: String,
        /// What specifically is missing.
        detail: &'static str,
    },

    /// A placement or mapped-item chain refers back to itself.
    ///
    /// The IFC spec pushes cycle prevention to the application layer, so real
    /// files do contain them. Detecting beats overflowing the stack.
    #[error("cyclic {kind} chain through {entity}")]
    CyclicChain {
        /// Where the cycle was detected.
        entity: EntityId,
        /// What kind of chain: `placement`, `mapped item`, ...
        kind: &'static str,
    },

    /// A chain exceeded its depth limit without closing.
    #[error("{kind} chain through {entity} exceeded depth {limit}")]
    ChainTooDeep {
        /// Where the walk gave up.
        entity: EntityId,
        /// What kind of chain.
        kind: &'static str,
        /// The limit that was hit.
        limit: usize,
    },

    /// An aggregate declared more elements than the caller's budget allows.
    ///
    /// IFC aggregate counts are file-controlled, so a small hostile file can
    /// declare an enormous element count and drive a large allocation before
    /// any geometric validation runs. This is that budget refusing, and it is
    /// always a refusal: nothing is truncated to fit.
    #[error("{entity} ({type_name}) declares {requested} {what}, over the limit of {limit}")]
    AggregateTooLarge {
        /// The offending entity.
        entity: EntityId,
        /// Its IFC type.
        type_name: String,
        /// What was being counted: `knot multiplicities`, `triangle indices`.
        what: &'static str,
        /// How many elements the file asked for.
        requested: u128,
        /// The budget that refused it.
        limit: usize,
    },

    /// A lowered product reached the mesh compiler, which refused it.
    ///
    /// Distinct from [`Self::Unsupported`]: lowering succeeded and the neutral
    /// DAG is valid IFC meaning. What failed is *execution* -- a provider that
    /// cannot evaluate this operation, or a budget that will not fund it. The
    /// compiler's own reason is preserved verbatim, because it names the
    /// missing capability precisely enough for a caller to register a provider
    /// for it. Only reachable with the `compile` feature.
    #[cfg(feature = "compile")]
    #[error("{entity} could not be compiled to a mesh: {reason}")]
    CompilationRefused {
        /// The product whose body representation was being compiled.
        entity: EntityId,
        /// The compiler's refusal, as reported by the provider.
        reason: String,
    },

    /// A volume was asked of a product whose compiled body is not a solid.
    ///
    /// Raised by `CompiledMesh::solid_mesh` for a surface model
    /// (`IfcShellBasedSurfaceModel`, `IfcFaceBasedSurfaceModel`) and for a
    /// backend that does not report closure. A surface has area, not
    /// volume; reporting the divergence sum of a closed shell the file never
    /// declared a solid would be a plausible, wrong number.
    #[cfg(feature = "compile")]
    #[error("{entity} is not a solid (closure {closure:?}); it has no volume")]
    NotASolid {
        /// The product whose body was compiled.
        entity: EntityId,
        /// The closure the backend reported: `Surface` or `Unknown`.
        closure: axiolid_mesh_compile_contract::MeshClosure,
    },

    /// An opening that voids a host could not be subtracted from it (#44).
    ///
    /// Raised only when a caller asked for NET geometry. Returning the gross
    /// body instead would make every net quantity downstream silently wrong,
    /// so the host is refused and the opening named. `cause` says why: the
    /// opening's body did not lower, it has no body, or the kernel refused the
    /// subtraction.
    #[error("opening {opening} could not be subtracted from {host}: {cause}")]
    OpeningNotSubtracted {
        /// The host whose net geometry was requested.
        host: EntityId,
        /// The voiding element that could not be removed.
        opening: EntityId,
        /// Why it could not be removed.
        #[source]
        cause: Box<GeometryError>,
    },

    /// The geometry is structurally impossible.
    ///
    /// A degenerate direction, a zero-radius circle, a self-referencing
    /// boolean. The file parses; the geometry does not exist.
    #[error("{entity} ({type_name}) is geometrically invalid: {detail}")]
    Degenerate {
        /// The offending entity.
        entity: EntityId,
        /// Its IFC type.
        type_name: String,
        /// Why it cannot be built.
        detail: String,
    },

    /// Units could not be resolved, so coordinates have no defined scale.
    #[error("unit resolution failed: {0}")]
    Units(String),

    /// An authored value was rejected before anything was staged (ADR 0011).
    ///
    /// Distinct from [`Self::Degenerate`], which describes an entity that
    /// already exists in a file. Here there is no entity yet and therefore no
    /// id to report: the caller passed a value that could not be written.
    #[error("cannot author {type_name}.{attribute}: {detail}")]
    InvalidAuthoredValue {
        /// The IFC type being authored.
        type_name: &'static str,
        /// The attribute whose value was rejected.
        attribute: &'static str,
        /// Why it cannot be written.
        detail: String,
    },
}

impl GeometryError {
    /// The entity this error is about, when there is one.
    ///
    /// Lets a caller collect failures per element rather than aborting a whole
    /// file for one bad wall.
    pub fn entity(&self) -> Option<EntityId> {
        match self {
            Self::MissingEntity { referrer, .. } => Some(*referrer),
            Self::MissingAttribute { entity, .. }
            | Self::WrongValueKind { entity, .. }
            | Self::WrongEntityType { entity, .. }
            | Self::Unsupported { entity, .. }
            | Self::CyclicChain { entity, .. }
            | Self::ChainTooDeep { entity, .. }
            | Self::AggregateTooLarge { entity, .. }
            | Self::Degenerate { entity, .. } => Some(*entity),
            #[cfg(feature = "compile")]
            Self::CompilationRefused { entity, .. } => Some(*entity),
            #[cfg(feature = "compile")]
            Self::NotASolid { entity, .. } => Some(*entity),
            // The opening is what failed; `host` stays readable on the variant.
            Self::OpeningNotSubtracted { opening, .. } => Some(*opening),
            Self::Units(_) | Self::InvalidAuthoredValue { .. } => None,
        }
    }

    /// Is this "valid IFC we do not handle yet" rather than a broken file?
    ///
    /// Callers building a viewer usually want to skip and count these, while
    /// treating genuine corruption differently.
    pub fn is_unsupported(&self) -> bool {
        match self {
            Self::Unsupported { .. } => true,
            // A net refusal is as supported as the reason behind it.
            Self::OpeningNotSubtracted { cause, .. } => cause.is_unsupported(),
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn errors_name_the_entity_so_failures_are_locatable() {
        let e = GeometryError::MissingAttribute {
            entity: EntityId(42),
            type_name: "IFCEXTRUDEDAREASOLID".into(),
            attribute: "SweptArea",
        };
        assert_eq!(e.entity(), Some(EntityId(42)));
        assert!(e.to_string().contains("#42"));
        assert!(e.to_string().contains("SweptArea"));
    }

    #[test]
    fn unsupported_is_distinguishable_from_corruption() {
        let unsupported = GeometryError::Unsupported {
            entity: EntityId(1),
            type_name: "IFCSECTIONEDSPINE".into(),
            detail: "spine interpolation",
        };
        let broken = GeometryError::Degenerate {
            entity: EntityId(1),
            type_name: "IFCCIRCLE".into(),
            detail: "zero radius".into(),
        };
        assert!(unsupported.is_unsupported());
        assert!(!broken.is_unsupported());
    }
}
