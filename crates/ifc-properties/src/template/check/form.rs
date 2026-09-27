//! What a template's `TemplateType` prescribes, per declared release.
//!
//! # Property forms
//!
//! IFC4 ADD2 TC1, `IfcSimplePropertyTemplateTypeEnum` ("This enumeration
//! defines the correct subtype of instances of IfcSimpleProperty or
//! IfcPhysicalSimpleQuantity"): each `P_*`/`Q_*` constant reads "The
//! properties defined by this IfcPropertyTemplate are of type
//! IfcPropertySingleValue" and so on, one entity each ([`SIMPLE`]).
//!
//! `IfcComplexPropertyTemplateTypeEnum` "defines the applicable subtype of
//! instances of IfcComplexProperty or IfcPhysicalComplexQuantity". It
//! documents `P_COMPLEX` as `IfcComplexProperty` and leaves `Q_COMPLEX`
//! blank; with two constants and two named entities, `Q_COMPLEX` is
//! `IfcPhysicalComplexQuantity`.
//!
//! IFC4X3 ADD2 adds `Q_NUMBER` and documents it only as "No description
//! available", while its `Q_TIME` row reads "of type IfcQuantityNumber"
//! (`lexical/IfcSimplePropertyTemplateTypeEnum.htm`). The checker takes
//! neither reading on trust: in IFC4X3 `Q_TIME` accepts `IfcQuantityTime`
//! or `IfcQuantityNumber`, and `Q_NUMBER` is undecided.
//!
//! An unset `TemplateType` prescribes only the template's own kind: any
//! simple property or simple quantity for an `IfcSimplePropertyTemplate`,
//! any complex property or complex quantity for an
//! `IfcComplexPropertyTemplate`.
//!
//! # Measure types
//!
//! IFC4 ADD2 TC1, `IfcSimplePropertyTemplate.PrimaryMeasureType` and
//! `SecondaryMeasureType` state, per template type, which value each
//! determines ([`measured`]). `PrimaryMeasureType` of `P_BOUNDEDVALUE`
//! determines `LowerBoundValue` and `SecondaryMeasureType` the
//! `UpperBoundValue`; the table value's are its defining and defined
//! values; `P_REFERENCEVALUE`'s is the referenced entity (the documentation
//! names `IfcPropertyTableValue.PropertyReference`, an attribute only
//! `IfcPropertyReferenceValue` has). No measure type is documented for the
//! quantity constants, or for `SetPointValue`, so none is checked.
//!
//! # Set templates
//!
//! IFC4 ADD2 TC1 `IfcPropertySetTemplate`: depending on `TemplateType` it is
//! a template for `IfcPropertySet` (`PSET_*`) or `IfcElementQuantity`
//! (`QTO_*`). `IfcPropertySetTemplateTypeEnum` states where such a set may
//! be assigned ([`attachment`]). IFC4X3 ADD2 adds `PSET_MATERIALDRIVEN` and
//! `PSET_PROFILEDRIVEN`, "to be encoded in an IfcMaterialProperties
//! [IfcProfileProperties] entity and assigned to an IfcMaterialDefinition
//! [IfcProfileDef]": neither is an `IfcPropertySetDefinition`, so no set an
//! `IfcRelDefinesByTemplate` can relate conforms, and no object may carry it.

use ifc_schema::SchemaVersion;

use super::super::layout::Layout;
use super::finding::MeasureRole;
use crate::template::PropertyTemplateKind;

/// `IfcSimplePropertyTemplateTypeEnum` constant to the entity it prescribes.
const SIMPLE: [(&str, &[&str]); 12] = [
    ("P_SINGLEVALUE", &["IFCPROPERTYSINGLEVALUE"]),
    ("P_ENUMERATEDVALUE", &["IFCPROPERTYENUMERATEDVALUE"]),
    ("P_BOUNDEDVALUE", &["IFCPROPERTYBOUNDEDVALUE"]),
    ("P_LISTVALUE", &["IFCPROPERTYLISTVALUE"]),
    ("P_TABLEVALUE", &["IFCPROPERTYTABLEVALUE"]),
    ("P_REFERENCEVALUE", &["IFCPROPERTYREFERENCEVALUE"]),
    ("Q_LENGTH", &["IFCQUANTITYLENGTH"]),
    ("Q_AREA", &["IFCQUANTITYAREA"]),
    ("Q_VOLUME", &["IFCQUANTITYVOLUME"]),
    ("Q_COUNT", &["IFCQUANTITYCOUNT"]),
    ("Q_WEIGHT", &["IFCQUANTITYWEIGHT"]),
    ("Q_TIME", &["IFCQUANTITYTIME"]),
];

/// IFC4X3 ADD2 `Q_TIME`: its own entity, or the one its row names.
const Q_TIME_IFC4X3: &[&str] = &["IFCQUANTITYTIME", "IFCQUANTITYNUMBER"];

/// `IfcComplexPropertyTemplateTypeEnum` constant to the entity it prescribes.
const COMPLEX: [(&str, &[&str]); 2] = [
    ("P_COMPLEX", &["IFCCOMPLEXPROPERTY"]),
    ("Q_COMPLEX", &["IFCPHYSICALCOMPLEXQUANTITY"]),
];

/// Any simple property or simple quantity.
const ANY_SIMPLE: &[&str] = &["IFCSIMPLEPROPERTY", "IFCPHYSICALSIMPLEQUANTITY"];
/// Any complex property or complex quantity.
const ANY_COMPLEX: &[&str] = &["IFCCOMPLEXPROPERTY", "IFCPHYSICALCOMPLEXQUANTITY"];

/// What a property template's `TemplateType` prescribes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Prescribed {
    /// A conforming property is one of these entities or a subtype.
    Entities(&'static [&'static str]),
    /// The constant is not a member of the release's enumeration.
    Unknown,
    /// The release documents no entity for the constant.
    Undocumented,
}

/// The entities a property of template `kind` and `template_type` may be.
pub(super) fn prescribed(
    layout: Layout,
    kind: PropertyTemplateKind,
    template_type: Option<&str>,
) -> Prescribed {
    let (entity, table, any): (_, &[(&str, &'static [&'static str])], _) = match kind {
        PropertyTemplateKind::Simple => ("IFCSIMPLEPROPERTYTEMPLATE", &SIMPLE, ANY_SIMPLE),
        PropertyTemplateKind::Complex => ("IFCCOMPLEXPROPERTYTEMPLATE", &COMPLEX, ANY_COMPLEX),
    };
    let Some(token) = template_type else {
        return Prescribed::Entities(any);
    };
    if !layout.enum_accepts(entity, "TemplateType", token) {
        return Prescribed::Unknown;
    }
    if layout.version() == SchemaVersion::Ifc4x3 && kind == PropertyTemplateKind::Simple {
        if token.eq_ignore_ascii_case("Q_NUMBER") {
            return Prescribed::Undocumented;
        }
        if token.eq_ignore_ascii_case("Q_TIME") {
            return Prescribed::Entities(Q_TIME_IFC4X3);
        }
    }
    table
        .iter()
        .find(|(constant, _)| constant.eq_ignore_ascii_case(token))
        .map_or(Prescribed::Undocumented, |(_, entities)| {
            Prescribed::Entities(entities)
        })
}

/// The attributes of a property of `template_type` whose values the
/// template's `measure` type determines.
pub(super) fn measured(template_type: &str, measure: MeasureRole) -> &'static [&'static str] {
    let upper = template_type.to_ascii_uppercase();
    match (upper.as_str(), measure) {
        ("P_SINGLEVALUE", MeasureRole::Primary) => &["NominalValue"],
        ("P_ENUMERATEDVALUE", MeasureRole::Primary) => &["EnumerationValues"],
        ("P_BOUNDEDVALUE", MeasureRole::Primary) => &["LowerBoundValue"],
        ("P_BOUNDEDVALUE", MeasureRole::Secondary) => &["UpperBoundValue"],
        ("P_LISTVALUE", MeasureRole::Primary) => &["ListValues"],
        ("P_TABLEVALUE", MeasureRole::Primary) => &["DefiningValues"],
        ("P_TABLEVALUE", MeasureRole::Secondary) => &["DefinedValues"],
        ("P_REFERENCEVALUE", MeasureRole::Primary) => &["PropertyReference"],
        _ => &[],
    }
}

/// Where a set of a set template's `TemplateType` may be assigned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Attachment {
    /// Anywhere (`NOTDEFINED`).
    Any,
    /// Only on subtypes of `IfcTypeObject`.
    Types,
    /// Only on subtypes of `IfcObject`.
    Objects,
    /// On subtypes of `IfcTypeObject` or of `IfcObject`.
    TypesOrObjects,
    /// Only on `IfcPerformanceHistory`.
    PerformanceHistory,
    /// On no object definition (IFC4X3 material- and profile-driven sets).
    Nowhere,
}

impl Attachment {
    /// Whether an object of IFC type `object` may carry such a set.
    pub(super) fn admits(self, layout: Layout, object: &str) -> bool {
        let is_type = || layout.is_a(object, "IFCTYPEOBJECT");
        let is_object = || layout.is_a(object, "IFCOBJECT");
        match self {
            Self::Any => true,
            Self::Types => is_type(),
            Self::Objects => is_object(),
            Self::TypesOrObjects => is_type() || is_object(),
            Self::PerformanceHistory => layout.is_a(object, "IFCPERFORMANCEHISTORY"),
            Self::Nowhere => false,
        }
    }
}

/// The attachment rule of set template type `token`, or `None` when it is
/// not a documented member of the release's enumeration.
pub(super) fn attachment(layout: Layout, token: &str) -> Option<Attachment> {
    if !layout.enum_accepts("IFCPROPERTYSETTEMPLATE", "TemplateType", token) {
        return None;
    }
    match token.to_ascii_uppercase().as_str() {
        "PSET_TYPEDRIVENONLY" | "QTO_TYPEDRIVENONLY" => Some(Attachment::Types),
        "PSET_TYPEDRIVENOVERRIDE" | "QTO_TYPEDRIVENOVERRIDE" => Some(Attachment::TypesOrObjects),
        "PSET_OCCURRENCEDRIVEN" | "QTO_OCCURRENCEDRIVEN" => Some(Attachment::Objects),
        "PSET_PERFORMANCEDRIVEN" => Some(Attachment::PerformanceHistory),
        "PSET_MATERIALDRIVEN" | "PSET_PROFILEDRIVEN" => Some(Attachment::Nowhere),
        "NOTDEFINED" => Some(Attachment::Any),
        _ => None,
    }
}

/// The set entity a set template type governs, if it names one.
pub(super) fn set_entity(token: &str) -> Option<&'static str> {
    let upper = token.to_ascii_uppercase();
    match upper.as_str() {
        "PSET_MATERIALDRIVEN" => Some("IFCMATERIALPROPERTIES"),
        "PSET_PROFILEDRIVEN" => Some("IFCPROFILEPROPERTIES"),
        _ if upper.starts_with("PSET_") => Some("IFCPROPERTYSET"),
        _ if upper.starts_with("QTO_") => Some("IFCELEMENTQUANTITY"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    //! The tables above are claims about the bundled releases; each is
    //! checked against the table it will be read with.

    use super::*;
    use ifc_model::Model;

    fn layout(token: &str) -> Layout {
        let mut model = Model::new();
        model.header_mut().schema = vec![token.to_owned()];
        Layout::permissive(&model)
    }

    #[test]
    fn every_property_template_type_prescribes_declared_entities() {
        for token in ["IFC4", "IFC4X3_ADD2"] {
            let layout = layout(token);
            for (kind, entity, enumeration) in [
                (
                    PropertyTemplateKind::Simple,
                    "IFCSIMPLEPROPERTYTEMPLATE",
                    "IfcSimplePropertyTemplateTypeEnum",
                ),
                (
                    PropertyTemplateKind::Complex,
                    "IFCCOMPLEXPROPERTYTEMPLATE",
                    "IfcComplexPropertyTemplateTypeEnum",
                ),
            ] {
                let members = layout.enum_members(enumeration);
                assert!(!members.is_empty(), "{token} declares {enumeration}");
                for member in members {
                    assert!(layout.enum_accepts(entity, "TemplateType", member));
                    match prescribed(layout, kind, Some(member)) {
                        Prescribed::Entities(entities) => {
                            for name in entities {
                                assert!(
                                    layout.schema().entity(name).is_some(),
                                    "{token}: {member} prescribes undeclared {name}"
                                );
                            }
                        }
                        Prescribed::Undocumented => assert!(
                            token == "IFC4X3_ADD2" && member == "Q_NUMBER",
                            "{token}: {member} has no documented entity"
                        ),
                        Prescribed::Unknown => panic!("{token}: {member} is a member"),
                    }
                }
            }
            for name in ANY_SIMPLE.iter().chain(ANY_COMPLEX) {
                assert!(layout.schema().entity(name).is_some(), "{token}: {name}");
            }
        }
        // Constants of one release are foreign to the other.
        assert_eq!(
            prescribed(
                layout("IFC4"),
                PropertyTemplateKind::Simple,
                Some("Q_NUMBER")
            ),
            Prescribed::Unknown
        );
    }

    #[test]
    fn every_measured_attribute_is_declared_by_the_prescribed_entity() {
        for token in ["IFC4", "IFC4X3_ADD2"] {
            let layout = layout(token);
            for (constant, entities) in SIMPLE {
                for measure in [MeasureRole::Primary, MeasureRole::Secondary] {
                    for attribute in measured(constant, measure) {
                        let declared = layout.schema().attribute_names(entities[0]);
                        assert!(
                            declared.iter().any(|name| name == attribute),
                            "{token}: {} has no {attribute}",
                            entities[0]
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn every_set_template_type_has_a_rule() {
        for token in ["IFC4", "IFC4X3_ADD2"] {
            let layout = layout(token);
            let members = layout.enum_members("IfcPropertySetTemplateTypeEnum");
            assert!(!members.is_empty());
            for member in members {
                assert!(attachment(layout, member).is_some(), "{token}: {member}");
                if member != "NOTDEFINED" {
                    let entity = set_entity(member).expect("a governed set entity");
                    assert!(
                        layout.schema().entity(entity).is_some(),
                        "{token}: {entity}"
                    );
                }
            }
            for name in ["IFCTYPEOBJECT", "IFCOBJECT", "IFCPERFORMANCEHISTORY"] {
                assert!(layout.schema().entity(name).is_some());
            }
        }
        assert_eq!(attachment(layout("IFC4"), "PSET_MATERIALDRIVEN"), None);
    }
}
