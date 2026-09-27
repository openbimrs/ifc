//! Exact, fail-closed IFC2X3/IFC4/IFC4X3 property resolution.
//!
//! Unlike the permissive views, this traversal rejects any incomplete or
//! malformed assignment data before it can claim an exact absence. Every
//! structural fact comes from the table of the one release the header
//! declares; nothing is aliased across releases.

mod assignment;
mod enumerate;
mod measure;
mod quantity;
mod refs;
mod release;
mod set;
mod unit;
mod value;

use std::{fmt, sync::Arc};

use ifc_model::{EntityId, Model};
use ifc_schema::SchemaVersion;

use assignment::assigned_sets;
pub use enumerate::{exact_properties, exact_properties_where, ExactPropertyEntry};
use release::validate_model;
use set::find_property;
pub use unit::{exact_unit, ExactUnit, ExactUnitError};

/// Provenance of an exact result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ExactSource {
    /// Resolved from a property set assigned directly to the occurrence
    /// via `IfcRelDefinesByProperties`.
    Occurrence,
    /// Resolved from a property set inherited through the occurrence's
    /// `IfcTypeObject`, identified by that type's entity id.
    Type(EntityId),
}

/// Exact IFC logical value without collapsing unknown into a boolean.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExactLogical {
    /// `IfcLogical` value `.F.`.
    False,
    /// `IfcLogical` value `.U.` — genuinely undetermined, not absent.
    Unknown,
    /// `IfcLogical` value `.T.`.
    True,
}

/// Scalar values accepted by the exact resolver.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ExactValue {
    /// The property carried no value (`$`), distinct from an absent property.
    Null,
    /// An `IFCBOOLEAN` payload.
    Bool(bool),
    /// An `IFCLOGICAL` payload, keeping the tri-state distinction.
    Logical(ExactLogical),
    /// An `IFCBINARY` payload, stored as its literal encoded text.
    Binary(Arc<str>),
    /// An `IFCINTEGER` payload.
    Integer(i64),
    /// An `IFCREAL` (or compatible measure) payload; always finite.
    Real(f64),
    /// An `IFCTEXT`/`IFCLABEL`/`IFCIDENTIFIER`-family string payload.
    Text(Arc<str>),
}

/// A uniquely resolved property with IFC identity and provenance.
#[derive(Debug, Clone, PartialEq)]
pub struct ExactProperty {
    /// Whether the value came from the occurrence or was inherited from its type.
    pub source: ExactSource,
    /// The owning `IfcPropertySet.Name` or `IfcElementQuantity.Name`.
    pub property_set: Arc<str>,
    /// Entity id of the `IfcPropertySet` or `IfcElementQuantity`.
    pub set_id: EntityId,
    /// Entity id of the `IfcPropertySingleValue`, or of the simple
    /// `IfcPhysicalQuantity` (e.g. `IfcQuantityLength`).
    pub property_id: EntityId,
    /// Declared IFC value type (for example `IFCINTEGER` or `IFCLENGTHMEASURE`).
    ///
    /// For a quantity it is the declared type of its value attribute in the
    /// bound release (`LengthValue : IfcLengthMeasure` gives
    /// `IFCLENGTHMEASURE`), since a quantity stores a bare number.
    pub value_type: Option<Arc<str>>,
    /// Explicit `IfcPropertySingleValue.Unit` or
    /// `IfcPhysicalSimpleQuantity.Unit`, if stated.
    pub unit_id: Option<EntityId>,
    /// The resolved `NominalValue`, or the quantity's value.
    pub value: ExactValue,
}

/// Exact lookup result.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ExactResolution {
    /// The property was found exactly once across occurrence and inherited sets.
    Present(ExactProperty),
    /// No occurrence or inherited property set or quantity set carried a
    /// matching property or quantity; this is a proven absence, not a lookup
    /// failure.
    ///
    /// A predefined property set (`IfcDoorLiningProperties` and the like)
    /// whose own attribute has the requested name is refused with
    /// [`ExactPropertyError::UnsupportedDefinition`], never skipped into an
    /// `Absent` (#66).
    Absent,
}

/// Why a property cannot be resolved exactly.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ExactPropertyError {
    /// The model carries STEP-level diagnostics, so exactness cannot be
    /// guaranteed; count of diagnostics is reported for context.
    IncompleteModel {
        /// Number of diagnostics recorded against the model.
        diagnostics: usize,
    },
    /// The model header declares no `FILE_SCHEMA`.
    MissingSchema,
    /// The model header declares more than one schema.
    MultipleSchemas {
        /// Number of schemas declared in the header.
        schemas: usize,
    },
    /// The model header declares a schema other than IFC2X3, IFC4 or
    /// IFC4X3 (`IFC4X3_ADD2`).
    UnsupportedSchema {
        /// The declared schema token.
        schema: String,
    },
    /// An entity reference points at an id absent from the model.
    MissingReference {
        /// The entity holding the reference.
        from: EntityId,
        /// The entity id that could not be resolved.
        to: EntityId,
    },
    /// An aggregate attribute (list/set) was `$`, empty when required, or
    /// not encoded as a list at all.
    MalformedAggregate {
        /// The entity whose attribute was malformed.
        entity: EntityId,
        /// Name of the offending attribute.
        attribute: &'static str,
    },
    /// The occurrence is assigned to more than one `IfcTypeObject` via
    /// `IfcRelDefinesByType`, which IFC forbids.
    MultipleTypeAssignments {
        /// The occurrence with conflicting type assignments.
        object: EntityId,
        /// The first `IfcTypeObject` found.
        first: EntityId,
        /// The second, conflicting `IfcTypeObject` found.
        second: EntityId,
    },
    /// `IfcRelDefinesByProperties.RelatedObjects` names an object that is
    /// not a non-type `IfcObjectDefinition`.
    InvalidOccurrenceTarget {
        /// The `IfcRelDefinesByProperties` relationship.
        relationship: EntityId,
        /// The invalid related object.
        object: EntityId,
    },
    /// `IfcRelDefinesByType.RelatedObjects` names an object that is not an
    /// `IfcObject`.
    InvalidTypeTarget {
        /// The `IfcRelDefinesByType` relationship.
        relationship: EntityId,
        /// The invalid related object.
        object: EntityId,
    },
    /// The queried entity is not a non-type `IfcObjectDefinition` and
    /// therefore cannot carry properties.
    InvalidQueryObject {
        /// The rejected query object.
        object: EntityId,
        /// The object's actual IFC type name.
        type_name: Arc<str>,
    },
    /// The same entity id appears more than once in an aggregate attribute
    /// that must have unique members.
    DuplicateAggregateMember {
        /// The entity holding the aggregate.
        entity: EntityId,
        /// Name of the offending attribute.
        attribute: &'static str,
        /// The entity id that appeared more than once.
        member: EntityId,
    },
    /// Two property sets with the same name matched the query for the same
    /// source (occurrence or type), making the result ambiguous.
    DuplicateMatchingSets {
        /// Whether the ambiguity arose among occurrence or type sets.
        source: ExactSource,
        /// The first matching property set.
        first: EntityId,
        /// The second, conflicting matching property set.
        second: EntityId,
    },
    /// Two properties with the same name matched within the same set.
    DuplicateMatchingProperties {
        /// The `IfcPropertySet` containing the ambiguous properties.
        set: EntityId,
        /// The first matching property.
        first: EntityId,
        /// The second, conflicting matching property.
        second: EntityId,
    },
    /// A `Name` attribute expected to be a string was `$`, a reference, or
    /// otherwise not text.
    MalformedName {
        /// The entity whose name attribute was malformed.
        entity: EntityId,
        /// Name of the offending attribute (normally `"Name"`).
        attribute: &'static str,
    },
    /// A `RelatingPropertyDefinition` reference resolves to an entity that
    /// is not an `IfcPropertySetDefinition`, or to a predefined property set
    /// whose own attribute carries the requested name: its value lives in an
    /// entity attribute this resolver does not read, so absence cannot be
    /// proven.
    UnsupportedDefinition {
        /// The rejected entity.
        entity: EntityId,
        /// The entity's actual IFC type name.
        type_name: Arc<str>,
    },
    /// A member of `IfcPropertySet.HasProperties` is not an `IfcProperty`,
    /// or is a property kind the exact resolver does not yet support.
    UnsupportedProperty {
        /// The rejected entity.
        entity: EntityId,
        /// The entity's actual IFC type name.
        type_name: Arc<str>,
    },
    /// `IfcPropertySingleValue.NominalValue` was `$` where a value was required.
    MissingValueSlot {
        /// The property with the missing value.
        property: EntityId,
    },
    /// An entity's attribute count does not match what the declared
    /// release's schema declares for its type (a malformed or truncated STEP record).
    MalformedEntitySlots {
        /// The malformed entity.
        entity: EntityId,
        /// The entity's IFC type name.
        type_name: Arc<str>,
        /// Attribute count the schema declares for this type.
        expected: usize,
        /// Attribute count actually present on the entity.
        actual: usize,
    },
    /// `IfcPropertySingleValue.Unit` references an entity that is not a
    /// member of the `IfcUnit` select.
    UnsupportedUnit {
        /// The property with the invalid unit reference.
        property: EntityId,
    },
    /// `IfcPropertySingleValue.NominalValue` carries a typed value whose
    /// declared type is not accepted by `IFCVALUE`, or whose payload does
    /// not match its declared type.
    UnsupportedValue {
        /// The property with the invalid value.
        property: EntityId,
    },
    /// `IfcPropertySingleValue.NominalValue` is an `IFCREAL` that is NaN or
    /// infinite, which IFC does not permit.
    NonFiniteReal {
        /// The property with the non-finite real value.
        property: EntityId,
    },
    /// A traversed record, typed value, or select member names a construct
    /// that the release declared in `FILE_SCHEMA` does not define, for
    /// example an `IfcDoorType` or an `IfcPropertySetDefinitionSet` in an
    /// IFC2X3 file. The file mixes releases, so nothing it says about the
    /// property is trusted.
    NotInSchema {
        /// The entity holding or being the foreign construct.
        entity: EntityId,
        /// The construct's name as written in the file.
        name: Arc<str>,
        /// The release the header declares.
        schema: SchemaVersion,
    },
    /// A proper subtype of `IfcRelDefinesByProperties` or
    /// `IfcRelDefinesByType` relates the queried object, such as IFC2X3
    /// `IfcRelOverridesProperties`. Its semantics change which value applies,
    /// and the exact resolver does not interpret them, so it refuses rather
    /// than answer as if the relationship were absent.
    UnsupportedRelationship {
        /// The relationship instance.
        relationship: EntityId,
        /// Its IFC type name.
        type_name: Arc<str>,
    },
}
impl fmt::Display for ExactPropertyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "exact IFC property resolution failed: {self:?}")
    }
}

/// The IFC release a model's properties are resolved against.
///
/// This is the release `exact_property` binds to: the single `FILE_SCHEMA`
/// token, if it names a release the exact resolver supports (IFC2X3 TC1,
/// IFC4 ADD2 TC1 or IFC4X3 ADD2, the bundled tables). A consumer binds its vocabulary per release with this answer
/// instead of re-parsing the header. It fails exactly as `exact_property`
/// fails at model level: diagnostics, no schema, several schemas, or an
/// unsupported one.
///
/// # Errors
///
/// [`ExactPropertyError::IncompleteModel`], [`ExactPropertyError::MissingSchema`],
/// [`ExactPropertyError::MultipleSchemas`], or
/// [`ExactPropertyError::UnsupportedSchema`].
pub fn exact_schema(model: &Model) -> Result<SchemaVersion, ExactPropertyError> {
    validate_model(model).map(|release| release.version)
}

impl std::error::Error for ExactPropertyError {}

/// Resolve an `IfcPropertySingleValue` or simple quantity by exact set and
/// property name.
///
/// The model is resolved against the single release its `FILE_SCHEMA`
/// declares, IFC2X3, IFC4 or IFC4X3 (see [`exact_schema`]); every domain,
/// select and slot count is that release's. With `set_name == None`, all
/// assigned sets are searched. Occurrence values override matching inherited
/// values at property level. To enumerate every property instead of naming
/// one, use [`exact_properties`].
///
/// Quantity sets are searched like property sets, as buildingSMART IDS
/// treats a quantity as a property: `Qto_WallBaseQuantities.Length` resolves
/// to the `IfcQuantityLength` of that name. A property set and a quantity set
/// of the same matching name on one source are ambiguous and refused.
/// WHERE rules such as `LengthValue >= 0` are not evaluated; the value is
/// the file's.
///
/// # Errors
///
/// Any [`ExactPropertyError`]: the answer is refused rather than guessed
/// whenever the evidence is incomplete, ambiguous, or foreign to the
/// declared release.
pub fn exact_property(
    model: &Model,
    object: EntityId,
    set_name: Option<&str>,
    property_name: &str,
) -> Result<ExactResolution, ExactPropertyError> {
    let release = validate_model(model)?;
    let assigned = assigned_sets(model, release, object)?;
    let occurrence = find_property(
        model,
        release,
        &assigned.occurrence_sets,
        ExactSource::Occurrence,
        set_name,
        property_name,
    )?;
    let inherited = match &assigned.type_sets {
        Some((type_id, sets)) => find_property(
            model,
            release,
            sets,
            ExactSource::Type(*type_id),
            set_name,
            property_name,
        )?,
        None => None,
    };
    Ok(occurrence
        .or(inherited)
        .map_or(ExactResolution::Absent, ExactResolution::Present))
}
