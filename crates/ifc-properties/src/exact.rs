//! Exact, fail-closed IFC2X3/IFC4/IFC4X3 property resolution.
//!
//! Unlike the permissive views, this traversal rejects any incomplete or
//! malformed assignment data before it can claim an exact absence. Every
//! structural fact comes from the table of the one release the header
//! declares; nothing is aliased across releases.
//!
//! # Exact versus permissive
//!
//! The permissive views (`property_set`, `quantity_sets`, `query`) are for
//! interactive inspection. A rule engine, validator or checker uses this
//! exact API and must never map one of its errors to "absent": an error
//! means the file could not prove the answer, which is not the same as the
//! property being missing.
//!
//! Exact values keep their declared IFC value type and explicit unit
//! identity. A downstream adapter that cannot project a value category or
//! unit losslessly into its own model must reject it rather than coerce it.

mod assignment;
mod complex;
mod composite;
mod enumerate;
mod material;
mod measure;
mod predefined;
mod quantity;
mod refs;
mod release;
mod set;
mod unit;
mod value;
mod values;

use std::{fmt, sync::Arc};

use ifc_model::{EntityId, Model};
use ifc_schema::SchemaVersion;

use assignment::assigned_sets;
pub use enumerate::{
    exact_properties, exact_properties_where, exact_property_sets_where, ExactPropertyEntry,
    ExactPropertySetEntry,
};
pub use material::{
    exact_material_properties_where, exact_material_property, exact_material_property_sets_where,
};
pub use predefined::{exact_predefined_sets, ExactPredefinedSet};
use release::validate_model;
use set::find_property;
pub use unit::{exact_unit, ExactUnit, ExactUnitError};
pub use values::{
    ExactBoundedValue, ExactComplexMember, ExactComplexValue, ExactEntityRef, ExactEnumeratedValue,
    ExactEnumeration, ExactReferenceValue, ExactTableRow, ExactTableValue, ExactTypedValue,
};

/// Provenance of an exact result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ExactSource {
    /// Resolved from a property set assigned directly to the occurrence
    /// via `IfcRelDefinesByProperties`.
    Occurrence,
    /// Resolved from a set in `HasPropertySets` of the `IfcTypeObject` with
    /// this entity id: inherited through the occurrence's type, or, when a
    /// type object is queried, that object's own set (#193), so the id is
    /// then the queried object's.
    Type(EntityId),
    /// Resolved from a material property set of the material definition
    /// with this entity id (#218): an `IfcMaterialProperties`, or in IFC2X3
    /// one of its subtypes, whose `Material` is that definition.
    Material(EntityId),
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

/// Values accepted by the exact resolver.
///
/// An `IfcPropertySingleValue` or a simple quantity resolves to one of the
/// scalar variants. The other `IfcSimpleProperty` kinds resolve to a
/// composite variant whose scalars carry their own declared types
/// ([`ExactTypedValue`]), and a complex property or quantity to
/// [`ExactValue::Complex`]; for those, [`ExactProperty::value_type`] is
/// `None`.
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
    /// An `IfcPropertyEnumeratedValue`: its selected values and its
    /// reference enumeration.
    Enumerated(ExactEnumeratedValue),
    /// An `IfcPropertyListValue`: its `ListValues` in file order, all of
    /// one declared type; empty only when the attribute is `$` (IFC4 and
    /// IFC4X3). The shared `Unit` is [`ExactProperty::unit_id`].
    List(Vec<ExactTypedValue>),
    /// An `IfcPropertyBoundedValue`: lower and upper bound and set point.
    Bounded(Box<ExactBoundedValue>),
    /// An `IfcPropertyTableValue`: its rows, expression, units and
    /// interpolation.
    Table(ExactTableValue),
    /// An `IfcPropertyReferenceValue`: its usage name and target entity.
    Reference(ExactReferenceValue),
    /// An enumeration constant held by a predefined set's attribute, as
    /// written without its dots, e.g. `SWINGING` for
    /// `IfcDoorPanelProperties.PanelOperation`; always a member of the
    /// declared enumeration in the bound release.
    Enum(Arc<str>),
    /// An entity held by a predefined set's attribute, e.g. the
    /// `IfcShapeAspect` of `ShapeAspectStyle`; checked, not followed.
    Entity(ExactEntityRef),
    /// An `IfcComplexProperty` or `IfcPhysicalComplexQuantity` (#208): its
    /// members, each resolved as a set member is. The complex is present
    /// but is no value of any type, so [`ExactProperty::value_type`] and
    /// [`ExactProperty::unit_id`] are `None`.
    Complex(ExactComplexValue),
}

/// A uniquely resolved property with IFC identity and provenance.
#[derive(Debug, Clone, PartialEq)]
pub struct ExactProperty {
    /// Whether the value came from the occurrence or from a type object's
    /// `HasPropertySets` (inherited, or the queried type object's own).
    pub source: ExactSource,
    /// The owning `IfcPropertySet.Name` or `IfcElementQuantity.Name`; for a
    /// predefined set its `Name`, or its entity name in the release's
    /// spelling (`IfcDoorLiningProperties`) when it states none.
    pub property_set: Arc<str>,
    /// Entity id of the `IfcPropertySet`, `IfcElementQuantity` or
    /// predefined set.
    pub set_id: EntityId,
    /// Entity id of the `IfcProperty` (single, enumerated, list, bounded,
    /// table or reference value, or complex), or of the
    /// `IfcPhysicalQuantity` (e.g. `IfcQuantityLength`, or a complex
    /// quantity). For an attribute of a predefined set, which is no entity
    /// of its own, the set's id.
    pub property_id: EntityId,
    /// Declared IFC value type (for example `IFCINTEGER` or `IFCLENGTHMEASURE`).
    ///
    /// `None` for a single value whose `NominalValue` is `$`, for a
    /// composite value, whose scalars carry their own types, and for a
    /// complex property or quantity, which has no value type at all.
    ///
    /// For a quantity it is the declared type of its value attribute in the
    /// bound release (`LengthValue : IfcLengthMeasure` gives
    /// `IFCLENGTHMEASURE`), since a quantity stores a bare number. For a
    /// predefined set's attribute it is the attribute's declared type
    /// (`LiningDepth : IfcPositiveLengthMeasure` gives
    /// `IFCPOSITIVELENGTHMEASURE`), also when the value is `$`, except that a
    /// select-typed attribute reports the member type the file wrote.
    pub value_type: Option<Arc<str>>,
    /// The explicit unit that applies to every value, if stated:
    /// `IfcPropertySingleValue.Unit`, `IfcPropertyListValue.Unit`,
    /// `IfcPropertyBoundedValue.Unit`, the `Unit` of an enumerated value's
    /// `IfcPropertyEnumeration`, or `IfcPhysicalSimpleQuantity.Unit`.
    /// A table's two units are in [`ExactTableValue`]; a reference value has
    /// none.
    pub unit_id: Option<EntityId>,
    /// The resolved `NominalValue`, composite value, or the quantity's value.
    pub value: ExactValue,
}

/// Exact lookup result.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ExactResolution {
    /// The property was found exactly once across occurrence and inherited sets.
    Present(ExactProperty),
    /// No occurrence or inherited property set, quantity set or predefined
    /// set carried a matching property, quantity or attribute; this is a
    /// proven absence, not a lookup failure.
    ///
    /// A predefined set's attribute that is `$` is not absent: it is
    /// `Present` with [`ExactValue::Null`].
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
    /// not a non-type `IfcObjectDefinition`. A type object there is refused
    /// even when it is the queried object: its sets belong in its
    /// `HasPropertySets` (IFC4 and IFC4X3 `NoRelatedTypeObject`; IFC2X3
    /// admits only `IfcObject`).
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
    /// The queried entity can carry no property sets in the declared
    /// release: it is neither an object `IfcRelDefinesByProperties` may
    /// relate (`IfcObject` in IFC2X3, a non-type `IfcObjectDefinition` in
    /// IFC4 and IFC4X3) nor an `IfcTypeObject` (accepted since #193). For
    /// a material query (#218): the entity is no material definition the
    /// release's `IfcMaterialProperties.Material` accepts.
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
    /// is not an `IfcPropertySetDefinition`; or a predefined property set
    /// holds the requested name in an attribute this resolver cannot read
    /// exactly (an aggregate, such as
    /// `IfcReinforcementDefinitionProperties.ReinforcementSectionDefinitions`);
    /// or a predefined set that states no `Name`, and so cannot be ruled out
    /// by a set name, has an attribute of the requested name (#66).
    UnsupportedDefinition {
        /// The rejected entity.
        entity: EntityId,
        /// The entity's actual IFC type name.
        type_name: Arc<str>,
    },
    /// A member of `IfcPropertySet.HasProperties` or
    /// `IfcComplexProperty.HasProperties` is not an `IfcProperty`, or a
    /// member of `IfcElementQuantity.Quantities` or
    /// `IfcPhysicalComplexQuantity.HasQuantities` is not an
    /// `IfcPhysicalQuantity`.
    UnsupportedProperty {
        /// The rejected entity.
        entity: EntityId,
        /// The entity's actual IFC type name.
        type_name: Arc<str>,
    },
    /// A value the release requires was `$`: a quantity's value, or a
    /// required value of another kind (IFC2X3
    /// `IfcPropertyReferenceValue.PropertyReference`).
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
    /// Values that the property's release constrains contradict one of
    /// those constraints, so no one reading of them is exact: list members
    /// or bounds of different types, table columns of unequal length, a
    /// selected value missing from the referenced enumeration, or a repeated
    /// member of a `LIST OF UNIQUE`.
    InconsistentValues {
        /// The property or `IfcPropertyEnumeration` holding the values.
        entity: EntityId,
        /// The release's label of the violated WHERE rule (for example
        /// `WR31`, or `SameUnitUpperLower` in IFC4), or `<Attribute> UNIQUE`
        /// for a repeated member of a unique list.
        rule: &'static str,
    },
    /// The entity name given to [`exact_predefined_sets`] is not a
    /// predefined property set in the declared release: not declared there
    /// at all, or an `IfcPropertySet`, a quantity set, or one of their
    /// supertypes.
    NotAPredefinedSet {
        /// The entity name as requested.
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
    /// A complex property or quantity reaches itself again through its
    /// members (#208). The schema forbids only a direct self-member; a
    /// longer cycle has no finite resolution either.
    ComplexCycle {
        /// The complex whose member list closes the cycle.
        complex: EntityId,
        /// The member already being resolved higher up the path.
        member: EntityId,
    },
    /// A complex property or quantity nested deeper than the resolver
    /// follows below one set member (#208).
    ComplexTooDeep {
        /// The complex whose members would exceed the depth.
        complex: EntityId,
        /// The nesting depth followed.
        limit: usize,
    },
    /// Resolving one set member followed more nested member references
    /// than its budget (#208). Members may be shared between complexes,
    /// so a small file can expand into an enormous tree.
    ComplexBudgetExceeded {
        /// The complex whose member exceeded the budget.
        complex: EntityId,
        /// The nested member references followed.
        limit: usize,
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

/// Resolve a property or quantity by exact set and property name.
///
/// Single, enumerated, list, bounded, table and reference values resolve,
/// the last five as composite [`ExactValue`]s. An `IfcComplexProperty` or
/// `IfcPhysicalComplexQuantity` resolves as [`ExactValue::Complex`]: present,
/// with no value type (#208).
///
/// A predefined property set (`IfcDoorLiningProperties` and the like) is
/// searched too: its members are the attributes its entity declares, by
/// schema name (`LiningDepth`), and its set name is its `Name`, or its
/// entity name (`IfcDoorLiningProperties`) when it states none. A door with
/// one `IfcDoorPanelProperties` per leaf is ambiguous here; list such sets
/// with [`exact_predefined_sets`].
///
/// The model is resolved against the single release its `FILE_SCHEMA`
/// declares, IFC2X3, IFC4 or IFC4X3 (see [`exact_schema`]); every domain,
/// select and slot count is that release's. With `set_name == None`, all
/// assigned sets are searched. Occurrence values override matching inherited
/// values at property level. To enumerate every property instead of naming
/// one, use [`exact_properties`].
///
/// `object` may also be an `IfcTypeObject` of the release (an `IfcWallType`,
/// an IFC2X3 `IfcDoorStyle`, ...). Its own `HasPropertySets` are then
/// searched, with the validation an occurrence's inherited type sets get,
/// and a result carries [`ExactSource::Type`] with `object`'s id. `$` states
/// no sets, a proven absence; an empty list is refused as for an inherited
/// type. A type object named in an `IfcRelDefinesByProperties` is refused
/// ([`ExactPropertyError::InvalidOccurrenceTarget`]), never ignored.
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
