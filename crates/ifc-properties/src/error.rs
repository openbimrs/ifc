//! Why a property lookup failed, and what a file got wrong.
//!
//! Anomalies describe a MALFORMED file, not a reader failure. They are
//! returned alongside results rather than replacing them: one broken
//! relationship must not hide every valid property in the model.

use ifc_model::EntityId;
use ifc_schema::SchemaVersion;

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
    /// Two properties of the same name in one property set, or two property
    /// templates of the same name in one set or complex template.
    ///
    /// Forbidden in IFC4 (`IfcPropertySet.UniquePropertyNames`) and IFC2X3
    /// (`WR32`). [`PropertySet::property`](crate::PropertySet::property)
    /// answers with the first in `HasProperties` order. For templates,
    /// `IfcPropertySetTemplate` and `IfcComplexPropertyTemplate` both carry
    /// `UniquePropertyNames`; `set` is then the template, and the first
    /// template in `HasPropertyTemplates` order is the one a check uses.
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
    /// A simple quantity states no value.
    ///
    /// The value attribute (`LengthValue`, `AreaValue`, ...) is not
    /// `OPTIONAL` on any `IfcQuantity*`. The quantity stays in its set's
    /// `quantities` as `Quantity::Unresolved` with
    /// `UnresolvedValue::Missing`, and is named here.
    QuantityValueMissing {
        /// The quantity entity.
        quantity: EntityId,
    },
    /// A simple quantity's value attribute holds something other than a
    /// number, such as text or a reference.
    ///
    /// The quantity stays in its set's `quantities` as
    /// `Quantity::Unresolved` with `UnresolvedValue::NotNumeric`, and is
    /// named here.
    QuantityValueNotNumeric {
        /// The quantity entity.
        quantity: EntityId,
        /// The value found, rendered for the message.
        found: String,
    },
    /// A property set, quantity set, complex property or complex quantity,
    /// or a property set or complex template, lists a member id that is not
    /// in the file.
    ///
    /// The member cannot be read, so it is absent from the resolved value.
    MissingMember {
        /// The set or complex entity listing the member.
        container: EntityId,
        /// The id it named.
        member: EntityId,
    },
    /// A member list holds an item that is not an entity reference.
    ///
    /// `IfcPropertySet.HasProperties`, `IfcComplexProperty.HasProperties`,
    /// `IfcElementQuantity.Quantities` and
    /// `IfcPhysicalComplexQuantity.HasQuantities` are sets of entity
    /// references. An item such as a string or number names no member, so
    /// nothing is read for it.
    MemberNotReference {
        /// The set or complex entity holding the list.
        container: EntityId,
        /// The list attribute, e.g. `"HasProperties"`.
        attribute: &'static str,
        /// The item found, rendered for the message.
        found: String,
    },
    /// A member list names the same entity more than once.
    ///
    /// Each of those lists is an EXPRESS `SET`, which cannot hold one
    /// instance twice. The member is read once, at its first position.
    DuplicateMember {
        /// The set or complex entity holding the list.
        container: EntityId,
        /// The list attribute, e.g. `"Quantities"`.
        attribute: &'static str,
        /// The member listed again.
        member: EntityId,
    },
    /// A complex property, complex quantity or complex property template
    /// reaches itself again through its members.
    ///
    /// The schema forbids only a DIRECT self-member (`IfcComplexProperty`
    /// `WR21`, `IfcPhysicalComplexQuantity.NoSelfReference`,
    /// `IfcComplexPropertyTemplate.NoSelfReference`); a longer cycle is just
    /// as unresolvable. `member` is already being read higher
    /// up the same path, so it is left out of `complex`'s resolved members.
    ComplexCycle {
        /// The complex entity whose member list closes the cycle.
        complex: EntityId,
        /// The member that re-enters the path.
        member: EntityId,
    },
    /// Complex nesting deeper than the reader follows.
    ///
    /// `complex` is resolved with its name and usage, but its members are
    /// not read.
    ComplexTooDeep {
        /// The complex entity whose members were not read.
        complex: EntityId,
        /// The nesting depth followed.
        limit: usize,
    },
    /// One read followed more nested member references than its budget.
    ///
    /// Members may legally be shared between complex properties, so a
    /// small file can expand into an enormous tree. Once the budget is
    /// spent, the remaining members of `complex` are not read.
    ComplexBudgetExceeded {
        /// The complex entity whose remaining members were not read.
        complex: EntityId,
        /// The nested member references followed.
        limit: usize,
    },
    /// A template's member list, or an `IfcRelDefinesByTemplate`, names an
    /// entity that is not a template of the required kind.
    ///
    /// `HasPropertyTemplates` is a `SET [1:?] OF IfcPropertyTemplate` and
    /// `RelatingTemplate` an `IfcPropertySetTemplate`. The entity is not
    /// read as a template.
    NotATemplate {
        /// The template or relationship naming it.
        container: EntityId,
        /// The entity named.
        member: EntityId,
        /// Its IFC type name.
        type_name: String,
    },
    /// A record's attribute count differs from what the bound release
    /// declares for its entity.
    ///
    /// Attributes are read by name from that release's table, so a short
    /// record would otherwise read its missing trailing attributes as unset.
    /// The attributes present are still read.
    SlotCountMismatch {
        /// The malformed record.
        entity: EntityId,
        /// Its IFC type name.
        type_name: String,
        /// The attribute count the release declares.
        expected: usize,
        /// The attribute count in the file.
        actual: usize,
    },
    /// An attribute holds a value its declared type does not admit: a
    /// number where a label is declared, an enumeration constant that is not
    /// a member of the release's enumeration, or a reference to an entity
    /// outside the declared type.
    ///
    /// The value is still reported as written where the view has a place
    /// for it (an enumeration constant, a reference id), and left unset
    /// otherwise.
    MalformedAttribute {
        /// The entity holding the attribute.
        entity: EntityId,
        /// The attribute, as the schema names it.
        attribute: &'static str,
        /// The value found, rendered for the message.
        found: String,
    },
}

/// Why property templates could not be read or checked against the
/// release a model declares.
///
/// Distinct from [`PropertyAnomaly`]: an anomaly is a malformed fact inside
/// a file that can still be read; this refuses the whole request.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum TemplateError {
    /// The model cannot be bound to one release, for the reason
    /// [`exact_schema`](crate::exact_schema) gives: STEP diagnostics, no or
    /// several `FILE_SCHEMA` entries, or an unsupported one.
    Release(crate::ExactPropertyError),
    /// The declared release defines no property templates.
    ///
    /// `IfcPropertySetTemplate` and its property templates are new in IFC4;
    /// IFC2X3 TC1 declares none of them, so nothing in such a file can be a
    /// template.
    NoTemplates {
        /// The release the header declares.
        schema: ifc_schema::SchemaVersion,
    },
    /// The entity is not in the model.
    MissingEntity {
        /// The id named by the caller.
        id: EntityId,
    },
    /// The entity is not the kind of template asked for.
    NotATemplate {
        /// The entity.
        id: EntityId,
        /// Its IFC type name.
        type_name: String,
    },
}

impl std::fmt::Display for TemplateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Release(error) => write!(f, "{error}"),
            Self::NoTemplates { schema } => {
                write!(f, "{schema:?} defines no property templates")
            }
            Self::MissingEntity { id } => write!(f, "#{} is not in the model", id.0),
            Self::NotATemplate { id, type_name } => {
                write!(f, "#{} is a {type_name}, not the template asked for", id.0)
            }
        }
    }
}

impl std::error::Error for TemplateError {}

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
    /// The model's header declares several schemas; authoring binds to
    /// exactly one release.
    MultipleSchemas {
        /// Number of `FILE_SCHEMA` declarations.
        schemas: usize,
    },
    /// The model's header declares one schema with no bundled table, so no
    /// layout can be trusted.
    UnsupportedSchema {
        /// The `FILE_SCHEMA` token as written.
        schema: String,
    },
    /// The model's release does not declare this entity, such as
    /// `IfcQuantityNumber` (IFC4X3 only) in an IFC2X3 or IFC4 model.
    EntityNotInSchema {
        /// The entity type, in schema casing.
        entity: &'static str,
        /// The release the model declares.
        schema: SchemaVersion,
    },
    /// An authoring call supplied a value for an attribute the model's
    /// release does not declare, such as a `Formula` for an IFC2X3
    /// quantity. It is refused rather than dropped.
    AuthoringNotInSchema {
        /// The entity type, in schema casing.
        entity: &'static str,
        /// The attribute.
        attribute: &'static str,
        /// The release the model declares.
        schema: SchemaVersion,
    },
    /// The record to edit does not have the attribute count its release
    /// declares, so no slot in it can be trusted.
    MalformedEntitySlots {
        /// The record.
        id: EntityId,
        /// Its type.
        type_name: String,
        /// Attribute count the release declares.
        expected: usize,
        /// Attribute count the record has.
        actual: usize,
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
            Self::MultipleSchemas { schemas } => write!(
                f,
                "the header declares {schemas} schemas; authoring binds to exactly one"
            ),
            Self::UnsupportedSchema { schema } => {
                write!(
                    f,
                    "the header declares {schema}, which has no bundled table"
                )
            }
            Self::EntityNotInSchema { entity, schema } => {
                write!(f, "{entity} is not an entity of {schema:?}")
            }
            Self::AuthoringNotInSchema {
                entity,
                attribute,
                schema,
            } => write!(
                f,
                "cannot author {entity}.{attribute}: not defined by {schema:?}"
            ),
            Self::MalformedEntitySlots {
                id,
                type_name,
                expected,
                actual,
            } => write!(
                f,
                "#{} {type_name} has {actual} attributes; its release declares {expected}",
                id.0
            ),
        }
    }
}

/// Result alias for authoring and reading helpers in this crate.
pub type PropertyResult<T> = Result<T, PropertyError>;

impl std::error::Error for PropertyError {}
