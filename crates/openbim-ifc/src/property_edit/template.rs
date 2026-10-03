//! What the release's PSD/QTO catalog (ADR 0017) says about a set.
//!
//! Only names with a buildingSMART prefix (`Pset_`, `Qto_`, any case) are
//! looked up: every catalog set has one, and an application's own set has
//! no template to be checked against. The catalog read is the release's own
//! edition, IFC2X3 TC1, IFC4 ADD2 TC1 or IFC4X3 ADD2, in its corrected
//! profile where one exists (IFC4, IFC4X3) and the official one otherwise.
//!
//! Without the `property-catalog` feature no template can be read, so a
//! prefixed set is reported as unavailable and the edit is refused rather
//! than written unchecked.

use std::collections::BTreeMap;

use ifc_properties::{QuantityKind, SchemaVersion};

use super::edit::PropertyEditFailure;

/// The value form a template member declares.
// Built only from a catalog entry.
#[cfg_attr(not(feature = "property-catalog"), allow(dead_code))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TemplateForm {
    /// `P_SINGLEVALUE`.
    Single,
    /// `P_ENUMERATEDVALUE`.
    Enumerated,
    /// `P_LISTVALUE`.
    List,
    /// A QTO quantity of this kind.
    Quantity(QuantityKind),
    /// A form this writer does not write (bounded, table, reference,
    /// complex), named for the refusal.
    Other(&'static str),
}

/// One member of a template.
#[derive(Debug, Clone)]
pub(super) struct MemberTemplate {
    pub(super) form: TemplateForm,
    /// The declared value type (`IfcLabel`), when the template states one.
    pub(super) data_type: Option<String>,
    /// The enumeration's members, when it lists any.
    pub(super) values: Vec<String>,
}

/// When a template's values may be stated.
#[cfg_attr(not(feature = "property-catalog"), allow(dead_code))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Context {
    /// On an occurrence or its type.
    Any,
    /// Only on a type object (`*_TYPEDRIVENONLY`).
    TypeOnly,
    /// Only on an occurrence (`*_OCCURRENCEDRIVEN`).
    OccurrenceOnly,
}

/// A set's template, summarized for the checks.
#[derive(Debug, Clone)]
pub(super) struct SetTemplate {
    pub(super) name: String,
    pub(super) quantity: bool,
    pub(super) context: Context,
    pub(super) members: BTreeMap<String, MemberTemplate>,
}

/// Whether `set` is named like a buildingSMART set.
pub(super) fn reserved(set: &str) -> bool {
    let prefix = |p: &str| {
        set.get(..p.len())
            .is_some_and(|s| s.eq_ignore_ascii_case(p))
    };
    prefix("Pset_") || prefix("Qto_")
}

/// The template of `set` in `version`'s catalog: `None` for a name the
/// catalog does not hold or one without a buildingSMART prefix.
///
/// # Errors
///
/// [`PropertyEditFailure::CatalogUnavailable`] for a prefixed name in a
/// build without `property-catalog`; [`PropertyEditFailure::InvalidModel`]
/// if the embedded catalog cannot be read.
pub(super) fn template(
    version: SchemaVersion,
    set: &str,
) -> Result<Option<SetTemplate>, PropertyEditFailure> {
    if !reserved(set) {
        return Ok(None);
    }
    lookup(version, set)
}

#[cfg(not(feature = "property-catalog"))]
fn lookup(_: SchemaVersion, set: &str) -> Result<Option<SetTemplate>, PropertyEditFailure> {
    Err(PropertyEditFailure::CatalogUnavailable(set.to_owned()))
}

#[cfg(feature = "property-catalog")]
fn lookup(version: SchemaVersion, set: &str) -> Result<Option<SetTemplate>, PropertyEditFailure> {
    use ifc_template_catalog::catalog::CatalogProfile;
    use ifc_template_catalog::definition::{
        CatalogEdition, PropertyKind, PropertySetType, QuantityKind as Qto, QuantitySetType,
        SetTemplateKind,
    };
    use ifc_template_catalog::embedded::load_catalog;

    let (edition, profile) = match version {
        SchemaVersion::Ifc2x3 => (CatalogEdition::Ifc2x3Tc1, CatalogProfile::Official),
        SchemaVersion::Ifc4 => (CatalogEdition::Ifc4Add2Tc1, CatalogProfile::Corrected),
        SchemaVersion::Ifc4x3 => (CatalogEdition::Ifc4x3Add2, CatalogProfile::Corrected),
        // The resolver binds no other release, so no edit reaches here
        // with one; refuse rather than read another edition's catalog.
        _ => return Ok(None),
    };
    let catalog = load_catalog(edition, profile).map_err(|error| {
        PropertyEditFailure::InvalidModel(format!(
            "the {edition:?} catalog cannot be read: {error}"
        ))
    })?;
    let Some(found) = catalog.get(set) else {
        return Ok(None);
    };
    let mut members = BTreeMap::new();
    let (quantity, context) = match &found.kind {
        SetTemplateKind::Property {
            set_type,
            properties,
        } => {
            for property in properties {
                let (form, data_type, values) = match &property.kind {
                    PropertyKind::SingleValue { data_type } => (
                        TemplateForm::Single,
                        data_type.type_name.clone(),
                        Vec::new(),
                    ),
                    PropertyKind::EnumeratedValue {
                        data_type,
                        values,
                        constants,
                        ..
                    } => (
                        TemplateForm::Enumerated,
                        data_type.as_ref().and_then(|d| d.type_name.clone()),
                        values
                            .iter()
                            .cloned()
                            .chain(constants.iter().map(|c| c.name.clone()))
                            .collect(),
                    ),
                    PropertyKind::ListValue { data_type } => {
                        (TemplateForm::List, data_type.type_name.clone(), Vec::new())
                    }
                    PropertyKind::BoundedValue { .. } => {
                        (TemplateForm::Other("a bounded value"), None, Vec::new())
                    }
                    PropertyKind::TableValue { .. } => {
                        (TemplateForm::Other("a table value"), None, Vec::new())
                    }
                    PropertyKind::ReferenceValue { .. } => {
                        (TemplateForm::Other("a reference value"), None, Vec::new())
                    }
                    _ => (TemplateForm::Other("a complex property"), None, Vec::new()),
                };
                members.insert(
                    property.name.clone(),
                    MemberTemplate {
                        form,
                        data_type,
                        values,
                    },
                );
            }
            let context = match set_type {
                PropertySetType::TypeDrivenOnly => Context::TypeOnly,
                PropertySetType::OccurrenceDriven => Context::OccurrenceOnly,
                _ => Context::Any,
            };
            (false, context)
        }
        SetTemplateKind::Quantity {
            set_type,
            quantities,
            ..
        } => {
            for quantity in quantities {
                let form = match quantity.kind {
                    Qto::Length => TemplateForm::Quantity(QuantityKind::Length),
                    Qto::Area => TemplateForm::Quantity(QuantityKind::Area),
                    Qto::Volume => TemplateForm::Quantity(QuantityKind::Volume),
                    Qto::Weight => TemplateForm::Quantity(QuantityKind::Weight),
                    Qto::Time => TemplateForm::Quantity(QuantityKind::Time),
                    Qto::Count => TemplateForm::Quantity(QuantityKind::Count),
                    Qto::Number => TemplateForm::Quantity(QuantityKind::Number),
                    _ => TemplateForm::Other("a quantity kind this writer does not know"),
                };
                members.insert(
                    quantity.name.clone(),
                    MemberTemplate {
                        form,
                        data_type: None,
                        values: Vec::new(),
                    },
                );
            }
            let context = match set_type {
                QuantitySetType::TypeDrivenOnly => Context::TypeOnly,
                QuantitySetType::OccurrenceDriven => Context::OccurrenceOnly,
                _ => Context::Any,
            };
            (true, context)
        }
        _ => return Ok(None),
    };
    Ok(Some(SetTemplate {
        name: found.name.clone(),
        quantity,
        context,
        members,
    }))
}
