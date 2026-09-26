//! Why a property lookup failed, and what a file got wrong.
//!
//! Anomalies describe a MALFORMED file, not a reader failure. They are
//! returned alongside results rather than replacing them: one broken
//! relationship must not hide every valid property in the model.

use ifc_model::EntityId;

/// A structural problem found while reading properties.
///
/// `#[non_exhaustive]`: new structural checks add variants without breaking
/// callers that match on this type.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum PropertyAnomaly {
    /// An object assigned two different types by `IfcRelDefinesByType`.
    ///
    /// IFC4 `IfcObject.IsTypedBy` is `SET [0:1]`; IFC2X3 `IfcObject` WR1
    /// allows at most one. The first relationship by id is kept, and only
    /// its type's sets are inherited.
    TypedTwice {
        /// The object with two types.
        object: EntityId,
        /// The type kept.
        kept: EntityId,
        /// The type rejected.
        rejected: EntityId,
        /// The `IfcRelDefinesByType` that was rejected.
        relation: EntityId,
    },
    /// Two property sets of the same name on one owner.
    ///
    /// IFC4 forbids it on both occurrences (`IfcObject.UniquePropertySetNames`)
    /// and types (`IfcTypeObject.UniquePropertySetNames`); IFC2X3 states no
    /// such rule. Either way the resolved view is keyed by name, so the set
    /// with the lower id is kept and the other is reported here rather than
    /// silently overwriting it.
    DuplicateSetName {
        /// The occurrence or type holding both sets.
        owner: EntityId,
        /// The set kept.
        kept: EntityId,
        /// The set not resolved.
        rejected: EntityId,
    },
    /// Two properties of the same name in one property set.
    ///
    /// Forbidden in IFC4 (`IfcPropertySet.UniquePropertyNames`) and IFC2X3
    /// (`WR32`). [`PropertySet::property`](crate::PropertySet::property)
    /// answers with the first in `HasProperties` order.
    DuplicatePropertyName {
        /// The property set.
        set: EntityId,
        /// The property kept.
        kept: EntityId,
        /// The property shadowed.
        rejected: EntityId,
    },
    /// An `IfcTypeObject` attached by `IfcRelDefinesByProperties`.
    ///
    /// Forbidden by the `NoRelatedTypeObject` WHERE rule: a type carries its
    /// sets in `HasPropertySets`. The set is still reported, because refusing
    /// to read a common exporter bug helps nobody.
    TypeAttachedByRelationship {
        /// The offending relationship.
        relationship: EntityId,
        /// The type object that must not be there.
        type_object: EntityId,
    },
    /// A relationship names a property definition absent from the file.
    MissingDefinition {
        /// The relationship.
        relationship: EntityId,
        /// The id it named.
        definition: EntityId,
    },
    /// A relationship names an object absent from the file.
    MissingObject {
        /// The relationship.
        relationship: EntityId,
        /// The id it named.
        object: EntityId,
    },
    /// A quantity states a unit whose type contradicts the quantity kind.
    ///
    /// `IfcQuantityLength.WR21` requires a LENGTHUNIT, and the sibling
    /// quantities carry the same rule. A file breaking it has stated two
    /// different things about the same number.
    QuantityUnitMismatch {
        /// The quantity entity.
        quantity: EntityId,
        /// The unit it named.
        unit: EntityId,
        /// The `IfcUnitEnum` the schema requires.
        expected: &'static str,
        /// The `IfcUnitEnum` the file stated.
        found: String,
    },
    /// A simple quantity states a negative value.
    ///
    /// Every `IfcQuantity*` carries `WR22 : Value >= 0.` (count included). A
    /// negative area is not a small error; it is a value no consumer should
    /// use for takeoff.
    NegativeQuantity {
        /// The quantity entity.
        quantity: EntityId,
        /// The value stated.
        value: f64,
    },
}

/// A refused authoring request.
///
/// Distinct from [`PropertyAnomaly`], which reports what a FILE got wrong.
/// These say the CALLER asked for something the schema does not allow, and
/// are returned before anything is staged so a rejected edit never reaches
/// a transaction.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum PropertyError {
    /// The entity is not in the model.
    MissingEntity {
        /// The id named by the caller.
        id: EntityId,
    },
    /// The entity is not a simple quantity type.
    NotAQuantity {
        /// The entity.
        id: EntityId,
        /// What it actually is.
        type_name: String,
    },
    /// The entity is not an `IfcElementQuantity`.
    NotAQuantitySet {
        /// The entity.
        id: EntityId,
        /// What it actually is.
        type_name: String,
    },
    /// An authored attribute value is not valid for its slot.
    ///
    /// Raised before staging, so a rejected draft never reaches the model.
    AuthoringInvalid {
        /// The entity type being authored.
        entity: &'static str,
        /// The attribute that failed.
        attribute: &'static str,
        /// The offending value, rendered for the message.
        value: String,
    },
}

impl std::fmt::Display for PropertyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingEntity { id } => write!(f, "#{} is not in the model", id.0),
            Self::NotAQuantity { id, type_name } => {
                write!(f, "#{} is a {type_name}, not a simple quantity", id.0)
            }
            Self::NotAQuantitySet { id, type_name } => {
                write!(f, "#{} is a {type_name}, not an IfcElementQuantity", id.0)
            }
            Self::AuthoringInvalid {
                entity,
                attribute,
                value,
            } => write!(
                f,
                "{entity}.{attribute} rejected the authored value: {value}"
            ),
        }
    }
}

/// Result alias for authoring and reading helpers in this crate.
pub type PropertyResult<T> = Result<T, PropertyError>;

impl std::error::Error for PropertyError {}
