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
//!
//! Release differences: IFC2X3 declares no `IfcMaterialRelationship` and an
//! ABSTRACT `IfcMaterialProperties`, so both are refused for an IFC2X3 model;
//! IFC4X3 renamed `IfcMaterialRelationship.Expression` to
//! `MaterialExpression` at the same position.

use ifc_model::{EntityId, Model, Transaction, Value};

use super::{invalid, optional_text, refs, require_accepts, require_exists, require_type};
use crate::error::MaterialResult;
use crate::release::Release;

/// Stage an `IfcMaterialProperties`. IFC4 onwards.
///
/// Properties attached to a material definition rather than to an
/// occurrence: density, conductivity, and the rest of a datasheet.
/// `Properties` is a `SET [1:?]`, so an empty set is malformed and
/// refused rather than written as an empty aggregate.
///
/// `Material` accepts any `IfcMaterialDefinition` subtype the model's
/// release declares, not only `IfcMaterial`: a layer, profile or
/// constituent can carry its own properties.
///
/// # Errors
///
/// Refuses an empty property set, a `Material` reference whose target is
/// not a material definition, and an IFC2X3 model
/// ([`crate::MaterialError::EntityNotInSchema`]), whose
/// `IfcMaterialProperties` is abstract.
pub fn create_material_properties(
    tx: &mut Transaction,
    model: &Model,
    name: Option<&str>,
    description: Option<&str>,
    properties: &[EntityId],
    material: EntityId,
) -> MaterialResult<EntityId> {
    const ENTITY: &str = "IFCMATERIALPROPERTIES";
    let release = Release::of(model);
    release.require_entity(ENTITY, None)?;
    if properties.is_empty() {
        return Err(invalid(
            ENTITY,
            "Properties",
            "expected at least one property",
        ));
    }
    for property in properties {
        require_exists(tx, model, *property)?;
    }
    require_accepts(tx, model, release, ENTITY, "Material", material)?;
    let record = release.record(
        ENTITY,
        vec![
            ("Name", optional_text(name)),
            ("Description", optional_text(description)),
            ("Properties", refs(properties)),
            ("Material", Value::Ref(material)),
        ],
    )?;
    Ok(tx.create(record))
}

/// Stage an `IfcMaterialRelationship`. IFC4 onwards.
///
/// Relates one material to the materials it is composed of or derived
/// from: a concrete mix to its cement and aggregate. `expression` records
/// the mix rule as authored prose (IFC4 `Expression`, IFC4X3
/// `MaterialExpression`), not something this crate evaluates.
///
/// # Errors
///
/// Refuses an empty `RelatedMaterials` set (`SET [1:?]`), a relating
/// material that is also among the related ones, any reference that is
/// not an `IfcMaterial`, and an IFC2X3 model.
pub fn create_material_relationship(
    tx: &mut Transaction,
    model: &Model,
    name: Option<&str>,
    description: Option<&str>,
    relating: EntityId,
    related: &[EntityId],
    expression: Option<&str>,
) -> MaterialResult<EntityId> {
    const ENTITY: &str = "IFCMATERIALRELATIONSHIP";
    let release = Release::of(model);
    release.require_entity(ENTITY, None)?;
    if related.is_empty() {
        return Err(invalid(
            ENTITY,
            "RelatedMaterials",
            "expected at least one related material",
        ));
    }
    if related.contains(&relating) {
        return Err(invalid(
            ENTITY,
            "RelatedMaterials",
            "a material cannot be derived from itself",
        ));
    }
    require_type(tx, model, release, relating, &["IFCMATERIAL"])?;
    for material in related {
        require_type(tx, model, release, *material, &["IFCMATERIAL"])?;
    }
    let record = release.record(
        ENTITY,
        vec![
            ("Name", optional_text(name)),
            ("Description", optional_text(description)),
            ("RelatingMaterial", Value::Ref(relating)),
            ("RelatedMaterials", refs(related)),
            ("Expression", optional_text(expression)),
        ],
    )?;
    Ok(tx.create(record))
}

/// Stage an `IfcMaterialClassificationRelationship`.
///
/// Classifies a material against external systems such as Uniclass or
/// OmniClass. `MaterialClassifications` is a `SET [1:?]` (of
/// `IfcClassificationSelect`, IFC2X3 `IfcClassificationNotationSelect`), so
/// an unclassified relationship is refused rather than written empty.
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
    const ENTITY: &str = "IFCMATERIALCLASSIFICATIONRELATIONSHIP";
    let release = Release::of(model);
    release.require_entity(ENTITY, None)?;
    if classifications.is_empty() {
        return Err(invalid(
            ENTITY,
            "MaterialClassifications",
            "expected at least one classification",
        ));
    }
    for classification in classifications {
        require_exists(tx, model, *classification)?;
    }
    require_type(tx, model, release, material, &["IFCMATERIAL"])?;
    let record = release.record(
        ENTITY,
        vec![
            ("MaterialClassifications", refs(classifications)),
            ("ClassifiedMaterial", Value::Ref(material)),
        ],
    )?;
    Ok(tx.create(record))
}

/// Stage an `IfcMaterialDefinitionRepresentation`.
///
/// Gives a material its presentation: the styled representations that
/// say how it draws. The schema states OnlyStyledRepresentations (IFC2X3
/// WR11), so every entry must be an `IfcStyledRepresentation` -- a surface
/// style hung on a plain `IfcShapeRepresentation` parses and then renders
/// as nothing.
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
    const ENTITY: &str = "IFCMATERIALDEFINITIONREPRESENTATION";
    let release = Release::of(model);
    release.require_entity(ENTITY, None)?;
    if representations.is_empty() {
        return Err(invalid(
            ENTITY,
            "Representations",
            "expected at least one styled representation",
        ));
    }
    for representation in representations {
        require_type(
            tx,
            model,
            release,
            *representation,
            &["IFCSTYLEDREPRESENTATION"],
        )?;
    }
    require_type(tx, model, release, material, &["IFCMATERIAL"])?;
    let record = release.record(
        ENTITY,
        vec![
            ("Name", optional_text(name)),
            ("Description", optional_text(description)),
            ("Representations", refs(representations)),
            ("RepresentedMaterial", Value::Ref(material)),
        ],
    )?;
    Ok(tx.create(record))
}
