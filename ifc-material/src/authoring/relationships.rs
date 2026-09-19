//! Authoring for the material relationship entities.
//!
//! Split from the parent module, which stages the material definitions
//! themselves: materials, layers, profiles, constituents and their sets.
//! This module stages the records that relate those definitions to each
//! other and to the wider model -- properties, composition, external
//! classification and presentation.
//!
//! Every SET here is `[1:?]` in the schema, so an empty aggregate is a
//! malformed record rather than an under-specified one, and is refused.

use ifc_model::{Entity, EntityId, Model, Transaction, Value};

use super::{invalid, optional_text, refs, require_exists, require_type};
use crate::error::MaterialResult;

/// `IfcMaterialDefinition` subtypes, which can carry their own properties.
const MATERIAL_DEFINITION_TYPES: &[&str] = &[
    "IFCMATERIAL",
    "IFCMATERIALLAYER",
    "IFCMATERIALLAYERSET",
    "IFCMATERIALPROFILE",
    "IFCMATERIALPROFILESET",
    "IFCMATERIALCONSTITUENT",
    "IFCMATERIALCONSTITUENTSET",
];

/// Stage an `IfcMaterialProperties`.
///
/// Properties attached to a material definition rather than to an
/// occurrence: density, conductivity, and the rest of a datasheet.
/// `Properties` is a `SET [1:?]`, so an empty set is malformed and
/// refused rather than written as an empty aggregate.
///
/// `Material` accepts any `IfcMaterialDefinition` subtype, not only
/// `IfcMaterial`: a layer, profile or constituent can carry its own
/// properties.
///
/// # Errors
///
/// Refuses an empty property set, and a `Material` reference whose
/// target is not a material definition.
pub fn create_material_properties(
    tx: &mut Transaction,
    model: &Model,
    name: Option<&str>,
    description: Option<&str>,
    properties: &[EntityId],
    material: EntityId,
) -> MaterialResult<EntityId> {
    if properties.is_empty() {
        return Err(invalid(
            "IFCMATERIALPROPERTIES",
            "Properties",
            "expected at least one property",
        ));
    }
    for property in properties {
        require_exists(tx, model, *property)?;
    }
    require_type(tx, model, material, MATERIAL_DEFINITION_TYPES)?;
    Ok(tx.create(Entity::new(
        "IFCMATERIALPROPERTIES",
        vec![
            optional_text(name),
            optional_text(description),
            refs(properties),
            Value::Ref(material),
        ],
    )))
}

/// Stage an `IfcMaterialRelationship`.
///
/// Relates one material to the materials it is composed of or derived
/// from: a concrete mix to its cement and aggregate. `MaterialExpression`
/// records the mix rule as authored prose, not something this crate
/// evaluates.
///
/// # Errors
///
/// Refuses an empty `RelatedMaterials` set (`SET [1:?]`), a relating
/// material that is also among the related ones, and any reference
/// that is not an `IfcMaterial`.
pub fn create_material_relationship(
    tx: &mut Transaction,
    model: &Model,
    name: Option<&str>,
    description: Option<&str>,
    relating: EntityId,
    related: &[EntityId],
    expression: Option<&str>,
) -> MaterialResult<EntityId> {
    if related.is_empty() {
        return Err(invalid(
            "IFCMATERIALRELATIONSHIP",
            "RelatedMaterials",
            "expected at least one related material",
        ));
    }
    if related.contains(&relating) {
        return Err(invalid(
            "IFCMATERIALRELATIONSHIP",
            "RelatedMaterials",
            "a material cannot be derived from itself",
        ));
    }
    require_type(tx, model, relating, &["IFCMATERIAL"])?;
    for material in related {
        require_type(tx, model, *material, &["IFCMATERIAL"])?;
    }
    Ok(tx.create(Entity::new(
        "IFCMATERIALRELATIONSHIP",
        vec![
            optional_text(name),
            optional_text(description),
            Value::Ref(relating),
            refs(related),
            optional_text(expression),
        ],
    )))
}

/// Stage an `IfcMaterialClassificationRelationship`.
///
/// Classifies a material against external systems such as Uniclass or
/// OmniClass. `MaterialClassifications` is a `SET [1:?]` of
/// `IfcClassificationSelect`, so an unclassified relationship is
/// refused rather than written empty.
///
/// # Errors
///
/// Refuses an empty classification set, and a `ClassifiedMaterial`
/// that is not an `IfcMaterial`.
pub fn create_material_classification_relationship(
    tx: &mut Transaction,
    model: &Model,
    classifications: &[EntityId],
    material: EntityId,
) -> MaterialResult<EntityId> {
    if classifications.is_empty() {
        return Err(invalid(
            "IFCMATERIALCLASSIFICATIONRELATIONSHIP",
            "MaterialClassifications",
            "expected at least one classification",
        ));
    }
    for classification in classifications {
        require_exists(tx, model, *classification)?;
    }
    require_type(tx, model, material, &["IFCMATERIAL"])?;
    Ok(tx.create(Entity::new(
        "IFCMATERIALCLASSIFICATIONRELATIONSHIP",
        vec![refs(classifications), Value::Ref(material)],
    )))
}

/// Stage an `IfcMaterialDefinitionRepresentation`.
///
/// Gives a material its presentation: the styled representations that
/// say how it draws. The schema states OnlyStyledRepresentations, so
/// every entry must be an `IfcStyledRepresentation` -- a surface style
/// hung on a plain `IfcShapeRepresentation` parses and then renders as
/// nothing.
///
/// # Errors
///
/// Refuses an empty representation list (`LIST [1:?]`), any entry that
/// is not an `IfcStyledRepresentation`, and a `RepresentedMaterial`
/// that is not an `IfcMaterial`.
pub fn create_material_definition_representation(
    tx: &mut Transaction,
    model: &Model,
    name: Option<&str>,
    description: Option<&str>,
    representations: &[EntityId],
    material: EntityId,
) -> MaterialResult<EntityId> {
    if representations.is_empty() {
        return Err(invalid(
            "IFCMATERIALDEFINITIONREPRESENTATION",
            "Representations",
            "expected at least one styled representation",
        ));
    }
    for representation in representations {
        require_type(tx, model, *representation, &["IFCSTYLEDREPRESENTATION"])?;
    }
    require_type(tx, model, material, &["IFCMATERIAL"])?;
    Ok(tx.create(Entity::new(
        "IFCMATERIALDEFINITIONREPRESENTATION",
        vec![
            optional_text(name),
            optional_text(description),
            refs(representations),
            Value::Ref(material),
        ],
    )))
}
