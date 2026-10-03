//! The configuration choices of the release XSDs that EXPRESS does not fix.
//!
//! ISO 10303-28 lets a configuration choose, per attribute, whether an
//! inverse attribute is written and whether the explicit attribute it
//! inverts is then left off its entity. The buildingSMART configuration
//! makes the same 21 choices in IFC4 ADD2 TC1 and IFC4X3 ADD2: the
//! relationship (`IfcRelAggregates`) is written inside the inverse of the
//! entity it relates (`IsDecomposedBy`), and its `RelatingObject` is not
//! written at all. A few derived subtypes restrict their supertype's
//! content and drop inverse elements with it. The reader accepts any
//! inverse, so it needs none of this; the writer needs all of it to place
//! each value where the XSD expects it. The tables hold names only, and the
//! `conformance` test checks them, element by element, against both XSDs.

use crate::typing::{EntityLayout, InverseLayout, XsdForm};
use ifc_schema::{Schema, SchemaVersion};

/// Explicit attributes the configuration leaves off their entity, as
/// `(declaring entity, attribute)`. Each is written through an inverse in
/// [`INVERSES`] instead, except `IfcRelDefinesByObject.RelatingObject`:
/// the configuration writes the inverse of its sibling `RelatedObjects`
/// (`IsDeclaredBy`), not `Declares`, so that attribute has no form at all.
const OMITTED: &[(&str, &str)] = &[
    ("IfcClassificationReference", "ReferencedSource"),
    ("IfcCoordinateOperation", "SourceCRS"),
    ("IfcGeometricRepresentationSubContext", "ParentContext"),
    ("IfcIndexedColourMap", "MappedTo"),
    ("IfcIndexedTextureMap", "MappedTo"),
    ("IfcMaterialDefinitionRepresentation", "RepresentedMaterial"),
    ("IfcMaterialProperties", "Material"),
    ("IfcProfileProperties", "ProfileDefinition"),
    ("IfcRelAggregates", "RelatingObject"),
    ("IfcRelContainedInSpatialStructure", "RelatingStructure"),
    ("IfcRelDeclares", "RelatingContext"),
    ("IfcRelDefinesByObject", "RelatingObject"),
    ("IfcRelDefinesByProperties", "RelatedObjects"),
    ("IfcRelDefinesByType", "RelatedObjects"),
    ("IfcRelFillsElement", "RelatingOpeningElement"),
    ("IfcRelNests", "RelatingObject"),
    ("IfcRelProjectsElement", "RelatingElement"),
    ("IfcRelReferencedInSpatialStructure", "RelatingStructure"),
    ("IfcRelVoidsElement", "RelatingBuildingElement"),
    ("IfcShapeAspect", "PartOfProductDefinitionShape"),
    ("IfcStyledItem", "Item"),
];

/// How an inverse attribute's element holds its relationships.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InverseForm {
    /// A container of relationship elements.
    Container,
    /// The element is the one relationship (`maxOccurs="1"`).
    Direct,
}

/// Inverse attributes the configuration writes, as `(declaring entity,
/// inverse, form)`. A direct inverse holds one relationship: `IFC4.xsd`
/// declares `maxOccurs="1"` even where EXPRESS allows a set
/// (`HasOpenings`). `IFC4X3_ADD2.xsd` declares `maxOccurs="?"` for three of
/// them, which is no bound at all; the writer holds them to one as well.
const INVERSES: &[(&str, &str, InverseForm)] = &[
    ("IfcClassification", "HasReferences", InverseForm::Container),
    (
        "IfcClassificationReference",
        "HasReferences",
        InverseForm::Container,
    ),
    ("IfcContext", "IsDefinedBy", InverseForm::Container),
    ("IfcContext", "Declares", InverseForm::Container),
    ("IfcElement", "HasProjections", InverseForm::Direct),
    ("IfcElement", "HasOpenings", InverseForm::Direct),
    (
        "IfcGeometricRepresentationContext",
        "HasSubContexts",
        InverseForm::Container,
    ),
    (
        "IfcGeometricRepresentationContext",
        "HasCoordinateOperation",
        InverseForm::Direct,
    ),
    ("IfcMaterial", "HasRepresentation", InverseForm::Direct),
    (
        "IfcMaterialDefinition",
        "HasProperties",
        InverseForm::Container,
    ),
    ("IfcObject", "IsDeclaredBy", InverseForm::Direct),
    ("IfcObject", "IsTypedBy", InverseForm::Direct),
    ("IfcObject", "IsDefinedBy", InverseForm::Container),
    ("IfcObjectDefinition", "IsNestedBy", InverseForm::Container),
    (
        "IfcObjectDefinition",
        "IsDecomposedBy",
        InverseForm::Container,
    ),
    ("IfcOpeningElement", "HasFillings", InverseForm::Direct),
    (
        "IfcProductDefinitionShape",
        "HasShapeAspects",
        InverseForm::Container,
    ),
    ("IfcProfileDef", "HasProperties", InverseForm::Container),
    ("IfcRepresentationItem", "StyledByItem", InverseForm::Direct),
    (
        "IfcSpatialElement",
        "ContainsElements",
        InverseForm::Container,
    ),
    (
        "IfcSpatialElement",
        "ReferencesElements",
        InverseForm::Container,
    ),
    ("IfcTessellatedFaceSet", "HasColours", InverseForm::Direct),
    (
        "IfcTessellatedFaceSet",
        "HasTextures",
        InverseForm::Container,
    ),
];

/// Inverse elements a derived subtype's restriction changes, as
/// `(restricted entity, inverse, form it keeps or None when dropped)`. An
/// XSD restriction restates its base's content; these drop or retype
/// inverse elements of a supertype for the subtype and its own subtypes.
fn restrictions(
    version: SchemaVersion,
) -> &'static [(&'static str, &'static str, Option<InverseForm>)] {
    match version {
        SchemaVersion::Ifc4 => &[
            (
                "IfcGeometricRepresentationSubContext",
                "HasSubContexts",
                None,
            ),
            (
                "IfcGeometricRepresentationSubContext",
                "HasCoordinateOperation",
                None,
            ),
            ("IfcMirroredProfileDef", "HasProperties", None),
            ("IfcOrientedEdge", "StyledByItem", None),
        ],
        SchemaVersion::Ifc4x3 => &[(
            "IfcOrientedEdge",
            "StyledByItem",
            Some(InverseForm::Container),
        )],
        _ => &[],
    }
}

/// Whether the configuration leaves `attribute` of `entity` off its element.
pub(crate) fn omitted(schema: &Schema, entity: &str, attribute: &str) -> bool {
    OMITTED
        .iter()
        .any(|(declarer, name)| *name == attribute && schema.is_a(entity, declarer))
}

/// One child element of an entity element, in content-model order.
#[derive(Debug, Clone)]
pub(crate) enum Element {
    /// An explicit attribute slot written as an element.
    Slot(usize),
    /// An inverse attribute holding relationships that point back.
    Inverse {
        inverse: InverseLayout,
        form: InverseForm,
    },
}

/// The child elements of an entity type's element, in the order its XSD
/// content model declares them: each supertype's first, root first, and
/// within one entity its explicit attributes, then its inverses.
pub(crate) fn elements(schema: &Schema, layout: &EntityLayout) -> Vec<Element> {
    let restricted = schema.version().map_or(&[][..], restrictions);
    let mut chain: Vec<&str> = schema.supertypes(&layout.name);
    chain.reverse();
    chain.push(&layout.name);
    let mut out = Vec::new();
    for level in chain.iter().filter_map(|name| schema.entity(name)) {
        for attribute in &level.attributes {
            let Some(slot) = layout.slot(&attribute.name) else {
                continue;
            };
            let declared = &layout.slots[slot];
            if declared.derived
                || declared.form == XsdForm::Attribute
                || omitted(schema, &layout.name, &attribute.name)
            {
                continue;
            }
            out.push(Element::Slot(slot));
        }
        for inverse in &level.inverses {
            let Some(mut form) = INVERSES
                .iter()
                .find(|(declarer, name, _)| *declarer == level.name && *name == inverse.name)
                .map(|(_, _, form)| *form)
            else {
                continue;
            };
            let restriction = restricted.iter().find(|(entity, name, _)| {
                *name == inverse.name && schema.is_a(&layout.name, entity)
            });
            if let Some((_, _, kept)) = restriction {
                match kept {
                    Some(kept) => form = *kept,
                    None => continue,
                }
            }
            let Some(inverse) = layout.inverse(&inverse.name) else {
                continue;
            };
            out.push(Element::Inverse {
                inverse: inverse.clone(),
                form,
            });
        }
    }
    out
}
