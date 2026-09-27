//! What a template check reports.

use std::sync::Arc;

use ifc_model::EntityId;

use crate::error::PropertyAnomaly;

/// Which measure-type attribute of an `IfcSimplePropertyTemplate` a value
/// was checked against.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum MeasureRole {
    /// `PrimaryMeasureType`.
    Primary,
    /// `SecondaryMeasureType`.
    Secondary,
}

/// Why a check could not decide whether a set conforms to its template.
///
/// Each reason names something IFC leaves undefined or the file leaves
/// unstated. The checker reports it rather than guessing either way.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum UndecidedReason {
    /// A `TemplateType` constant that is not a member of the declared
    /// release's enumeration, e.g. `Q_NUMBER` in an IFC4 file.
    UnknownTemplateType {
        /// The constant as written.
        value: Arc<str>,
    },
    /// A `TemplateType` constant whose documentation in the declared release
    /// does not say which entity it governs: IFC4X3 ADD2 documents
    /// `Q_NUMBER` with "No description available".
    UndocumentedTemplateType {
        /// The constant.
        value: Arc<str>,
    },
    /// A `PrimaryMeasureType` or `SecondaryMeasureType` that names no type
    /// or entity the declared release declares.
    UnknownMeasureType {
        /// Which of the two.
        measure: MeasureRole,
        /// The label as written.
        value: Arc<str>,
    },
    /// A value whose type cannot be read: a select member written without
    /// its type, or a reference to an entity absent from the file.
    UntypedValue {
        /// The property attribute holding it, e.g. `NominalValue`.
        attribute: &'static str,
    },
    /// A property template without a `Name`, which cannot be matched to any
    /// property: templates are matched to properties by `Name` alone.
    UnnamedTemplate,
    /// An `ApplicableEntity` entry that names no entity of the declared
    /// release, or does not follow the documented form
    /// `IfcEntity[/PREDEFINEDTYPE][[PerformanceHistory]]`.
    UnknownApplicableEntity {
        /// The entry, trimmed.
        entry: Arc<str>,
    },
    /// An `ApplicableEntity` entry qualified by a predefined type, on an
    /// object of that entity whose own `PredefinedType` is unset,
    /// `NOTDEFINED`, or not an attribute of its entity. The type object may
    /// state it instead, and that is not followed here.
    PredefinedTypeUnstated {
        /// The entry, trimmed.
        entry: Arc<str>,
    },
    /// An `ApplicableEntity` entry qualified by `[PerformanceHistory]` on an
    /// `IfcPerformanceHistory`: the entry names the object the history is
    /// assigned to by `IfcRelAssignsToControl`, which is not followed here.
    PerformanceHistory {
        /// The entry, trimmed.
        entry: Arc<str>,
    },
    /// The templated set is a predefined property set (e.g.
    /// `IfcDoorLiningProperties`), whose members are fixed attributes of its
    /// entity rather than named properties; templates are documented for
    /// `IfcPropertySet` and `IfcElementQuantity` only.
    PredefinedSet {
        /// The set's IFC type name.
        found: Arc<str>,
    },
}

/// One way a templated property set deviates from an
/// `IfcPropertySetTemplate` linked to it by `IfcRelDefinesByTemplate`.
///
/// Every finding names the set and the set template; findings about one
/// property also name the property and/or the (possibly nested) property
/// template. A finding is a deviation from the in-file template, which is
/// what the caller asked about; except where noted, IFC does not make it a
/// schema violation.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum TemplateFinding {
    /// A property template has no property of its `Name` in the set (or, for
    /// a nested template, in the complex property).
    MissingProperty {
        /// The templated set.
        set: EntityId,
        /// The `IfcPropertySetTemplate`.
        template: EntityId,
        /// The set, or the complex property or quantity whose members were
        /// compared.
        container: EntityId,
        /// The property template with no property.
        property_template: EntityId,
        /// Its `Name`.
        name: Arc<str>,
    },
    /// A property whose `Name` no property template of the set template (or
    /// of the complex template) states.
    UnexpectedProperty {
        /// The templated set.
        set: EntityId,
        /// The `IfcPropertySetTemplate`.
        template: EntityId,
        /// The set, or the complex property or quantity holding it.
        container: EntityId,
        /// The property or quantity.
        property: EntityId,
        /// Its `Name`.
        name: Arc<str>,
    },
    /// A property whose entity is not the one its template's `TemplateType`
    /// prescribes, e.g. an `IfcPropertySingleValue` where the template says
    /// `P_ENUMERATEDVALUE`.
    WrongForm {
        /// The templated set.
        set: EntityId,
        /// The `IfcPropertySetTemplate`.
        template: EntityId,
        /// The property or quantity.
        property: EntityId,
        /// Its template.
        property_template: EntityId,
        /// The template's `TemplateType`; `None` when unset, when any simple
        /// (for a simple template) or complex (for a complex one) property
        /// or quantity is expected.
        template_type: Option<Arc<str>>,
        /// The entities any of which would conform (subtypes included).
        expected: &'static [&'static str],
        /// The property's IFC type name.
        found: Arc<str>,
    },
    /// A value whose declared type is not the template's measure type.
    WrongMeasureType {
        /// The templated set.
        set: EntityId,
        /// The `IfcPropertySetTemplate`.
        template: EntityId,
        /// The property.
        property: EntityId,
        /// Its template.
        property_template: EntityId,
        /// Which measure type of the template applies.
        measure: MeasureRole,
        /// The property attribute checked, e.g. `NominalValue`.
        attribute: &'static str,
        /// The template's measure type, as written.
        expected: Arc<str>,
        /// The first nonconforming value's type, or the referenced entity's
        /// type for a reference value.
        found: Arc<str>,
    },
    /// The set is not the entity the template's `TemplateType` governs: a
    /// `PSET_*` template on an `IfcElementQuantity`, a `QTO_*` template on
    /// an `IfcPropertySet`, or (IFC4X3) a material- or profile-driven
    /// template on any property set definition.
    WrongSetKind {
        /// The templated set.
        set: EntityId,
        /// The `IfcPropertySetTemplate`.
        template: EntityId,
        /// The template's `TemplateType`.
        template_type: Arc<str>,
        /// The entity the template type governs.
        expected: &'static str,
        /// The set's IFC type name.
        found: Arc<str>,
    },
    /// The set sits on an object its template's `TemplateType` excludes: a
    /// `*_TYPEDRIVENONLY` set on an occurrence, an `*_OCCURRENCEDRIVEN` set
    /// on a type, a `PSET_PERFORMANCEDRIVEN` set on anything but an
    /// `IfcPerformanceHistory`.
    WrongAttachment {
        /// The templated set.
        set: EntityId,
        /// The `IfcPropertySetTemplate`.
        template: EntityId,
        /// The object carrying the set.
        object: EntityId,
        /// The template's `TemplateType`.
        template_type: Arc<str>,
        /// The object's IFC type name.
        found: Arc<str>,
    },
    /// The set sits on an object that no entry of the template's
    /// `ApplicableEntity` admits.
    OutsideApplicableEntity {
        /// The templated set.
        set: EntityId,
        /// The `IfcPropertySetTemplate`.
        template: EntityId,
        /// The object carrying the set.
        object: EntityId,
        /// The object's IFC type name.
        found: Arc<str>,
        /// The template's `ApplicableEntity`, as written.
        applicable_entity: Arc<str>,
    },
    /// Whether `subject` conforms cannot be decided from the file and the
    /// release's documentation.
    Undecided {
        /// The templated set.
        set: EntityId,
        /// The `IfcPropertySetTemplate`.
        template: EntityId,
        /// What could not be decided: a property, a property template, the
        /// set template, or an object carrying the set.
        subject: EntityId,
        /// Why.
        reason: UndecidedReason,
    },
}

/// Everything a template check found.
#[derive(Debug, Clone, PartialEq, Default)]
#[non_exhaustive]
pub struct TemplateReport {
    /// Deviations, grouped by set (ascending id), then template (ascending
    /// id); within one pair, the set's kind, then its members in file
    /// order, then its missing members in template order, then the objects
    /// carrying it (ascending id).
    pub findings: Vec<TemplateFinding>,
    /// Malformed facts met while reading templates, sets and relationships.
    /// A member or template left out because of one is not compared, so a
    /// report with anomalies is not a proof of conformance.
    pub anomalies: Vec<PropertyAnomaly>,
}
