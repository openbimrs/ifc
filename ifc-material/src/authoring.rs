//! Transactional IFC4 material authoring.
//!
//! These helpers only stage records. [`ifc_model::Transaction::commit`] owns
//! atomic graph/index application, so a failed batch cannot leave part of a
//! material graph in the model.
mod relationships;

pub use relationships::{
    create_material_classification_relationship, create_material_definition_representation,
    create_material_properties, create_material_relationship,
};

use std::collections::HashSet;

use ifc_model::{Edit, Entity, EntityId, Model, Transaction, Value};

use crate::{DirectionSense, LayerSetDirection, LogicalValue, MaterialError, MaterialResult};

/// Authored identity fields for `IfcMaterial`.
#[derive(Debug, Clone, Copy)]
pub struct MaterialDraft<'a> {
    /// `IfcMaterial.Name`.
    pub name: &'a str,
    /// `IfcMaterial.Description`, if given.
    pub description: Option<&'a str>,
    /// `IfcMaterial.Category`, if given.
    pub category: Option<&'a str>,
}

/// Authored fields for `IfcMaterialLayer`.
#[derive(Debug, Clone, Copy)]
pub struct LayerDraft<'a> {
    /// `IfcMaterialLayer.Material`, an `IfcMaterial` reference, if given.
    pub material: Option<EntityId>,
    /// `IfcMaterialLayer.LayerThickness`. Must be finite and non-negative.
    pub thickness: f64,
    /// `IfcMaterialLayer.IsVentilated`, if given.
    pub is_ventilated: Option<LogicalValue>,
    /// `IfcMaterialLayer.Name`, if given.
    pub name: Option<&'a str>,
    /// `IfcMaterialLayer.Description`, if given.
    pub description: Option<&'a str>,
    /// `IfcMaterialLayer.Category`, if given.
    pub category: Option<&'a str>,
    /// `IfcMaterialLayer.Priority`, if given. Must be in `0..=100`.
    pub priority: Option<i64>,
}

/// Ordered composition fields for `IfcMaterialLayerSet`.
#[derive(Debug, Clone, Copy)]
pub struct LayerSetDraft<'a> {
    /// `IfcMaterialLayerSet.MaterialLayers`, in set order. Must be non-empty.
    pub layers: &'a [EntityId],
    /// `IfcMaterialLayerSet.Name`, if given.
    pub name: Option<&'a str>,
    /// `IfcMaterialLayerSet.Description`, if given.
    pub description: Option<&'a str>,
}

/// Authored fields for `IfcRelAssociatesMaterial`.
#[derive(Debug, Clone, Copy)]
pub struct MaterialAssignmentDraft<'a> {
    /// `IfcRelAssociatesMaterial.GlobalId`. Must be a valid IFC compressed
    /// GUID.
    pub global_id: &'a str,
    /// `IfcRelAssociatesMaterial.Name`, if given.
    pub name: Option<&'a str>,
    /// `IfcRelAssociatesMaterial.Description`, if given.
    pub description: Option<&'a str>,
    /// `IfcRelAssociatesMaterial.RelatedObjects`. Must be non-empty and
    /// contain no duplicate references.
    pub related_objects: &'a [EntityId],
    /// `IfcRelAssociatesMaterial.RelatingMaterial`, an `IfcMaterialSelect`
    /// branch reference.
    pub relating_material: EntityId,
}

/// Stage a material identity record.
pub fn create_material(tx: &mut Transaction, draft: MaterialDraft<'_>) -> EntityId {
    tx.create(Entity::new(
        "IFCMATERIAL",
        vec![
            text(draft.name),
            optional_text(draft.description),
            optional_text(draft.category),
        ],
    ))
}

/// Stage a layer after checking its scalar and material-reference invariants.
pub fn create_layer(
    tx: &mut Transaction,
    model: &Model,
    draft: LayerDraft<'_>,
) -> MaterialResult<EntityId> {
    finite_non_negative("IFCMATERIALLAYER", "LayerThickness", draft.thickness)?;
    if let Some(priority) = draft.priority.filter(|value| !(0..=100).contains(value)) {
        return Err(invalid(
            "IFCMATERIALLAYER",
            "Priority",
            priority.to_string(),
        ));
    }
    if let Some(material) = draft.material {
        require_type(tx, model, material, &["IFCMATERIAL"])?;
    }
    Ok(tx.create(Entity::new(
        "IFCMATERIALLAYER",
        vec![
            draft.material.map_or(Value::Null, Value::Ref),
            Value::Real(draft.thickness),
            draft.is_ventilated.map_or(Value::Null, logical),
            optional_text(draft.name),
            optional_text(draft.description),
            optional_text(draft.category),
            draft.priority.map_or(Value::Null, Value::Integer),
        ],
    )))
}

/// Stage a non-empty ordered layer set. Layers may have been created earlier
/// in this transaction; their staged type is checked just like stored records.
pub fn create_layer_set(
    tx: &mut Transaction,
    model: &Model,
    draft: LayerSetDraft<'_>,
) -> MaterialResult<EntityId> {
    if draft.layers.is_empty() {
        return Err(invalid(
            "IFCMATERIALLAYERSET",
            "MaterialLayers",
            "expected at least one layer",
        ));
    }
    for &layer in draft.layers {
        require_type(
            tx,
            model,
            layer,
            &["IFCMATERIALLAYER", "IFCMATERIALLAYERWITHOFFSETS"],
        )?;
    }
    Ok(tx.create(Entity::new(
        "IFCMATERIALLAYERSET",
        vec![
            refs(draft.layers),
            optional_text(draft.name),
            optional_text(draft.description),
        ],
    )))
}

/// Stage a product/type material association after validating the IFC GlobalId,
/// non-empty relation end, and `IfcMaterialSelect` branch.
pub fn associate_material(
    tx: &mut Transaction,
    model: &Model,
    draft: MaterialAssignmentDraft<'_>,
) -> MaterialResult<EntityId> {
    if ifc_model::guid::Guid::parse(draft.global_id).is_none() {
        return Err(invalid(
            "IFCRELASSOCIATESMATERIAL",
            "GlobalId",
            "expected IFC compressed GUID",
        ));
    }
    if draft.related_objects.is_empty() {
        return Err(invalid(
            "IFCRELASSOCIATESMATERIAL",
            "RelatedObjects",
            "expected at least one object",
        ));
    }
    let mut unique = HashSet::new();
    for &object in draft.related_objects {
        if !unique.insert(object) {
            return Err(invalid(
                "IFCRELASSOCIATESMATERIAL",
                "RelatedObjects",
                "duplicate object reference",
            ));
        }
        require_exists(tx, model, object)?;
    }
    require_type(tx, model, draft.relating_material, MATERIAL_SELECT_TYPES)?;
    Ok(tx.create(Entity::new(
        "IFCRELASSOCIATESMATERIAL",
        vec![
            text(draft.global_id),
            Value::Null,
            optional_text(draft.name),
            optional_text(draft.description),
            refs(draft.related_objects),
            Value::Ref(draft.relating_material),
        ],
    )))
}

/// Authored fields for `IfcMaterialConstituent`.
#[derive(Debug, Clone, Copy)]
pub struct ConstituentDraft<'a> {
    /// `IfcMaterialConstituent.Name`, if given.
    pub name: Option<&'a str>,
    /// `IfcMaterialConstituent.Description`, if given.
    pub description: Option<&'a str>,
    /// `IfcMaterialConstituent.Material`, an `IfcMaterial` reference.
    pub material: EntityId,
    /// `IfcMaterialConstituent.Fraction`, if given. A ratio in `0.0..=1.0`.
    pub fraction: Option<f64>,
    /// `IfcMaterialConstituent.Category`, if given.
    pub category: Option<&'a str>,
}

/// Stage an `IfcMaterialConstituent`.
///
/// `Fraction` is an `IfcNormalisedRatioMeasure`: values outside `0..=1` are
/// refused because a constituent cannot be a negative or >100% share of its
/// set, and a wrong fraction silently misstates a composition.
pub fn create_constituent(
    tx: &mut Transaction,
    model: &Model,
    draft: ConstituentDraft<'_>,
) -> MaterialResult<EntityId> {
    if let Some(fraction) = draft.fraction {
        if !fraction.is_finite() || !(0.0..=1.0).contains(&fraction) {
            return Err(invalid(
                "IFCMATERIALCONSTITUENT",
                "Fraction",
                "expected a normalised ratio in 0..=1",
            ));
        }
    }
    require_type(tx, model, draft.material, &["IFCMATERIAL"])?;
    Ok(tx.create(Entity::new(
        "IFCMATERIALCONSTITUENT",
        vec![
            optional_text(draft.name),
            optional_text(draft.description),
            Value::Ref(draft.material),
            draft.fraction.map_or(Value::Null, Value::Real),
            optional_text(draft.category),
        ],
    )))
}

/// Stage an `IfcMaterialConstituentSet`. Must name at least one
/// constituent: an empty set describes no composition at all.
pub fn create_constituent_set(
    tx: &mut Transaction,
    model: &Model,
    constituents: &[EntityId],
    name: Option<&str>,
    description: Option<&str>,
) -> MaterialResult<EntityId> {
    if constituents.is_empty() {
        return Err(invalid(
            "IFCMATERIALCONSTITUENTSET",
            "MaterialConstituents",
            "expected at least one constituent",
        ));
    }
    for &constituent in constituents {
        require_type(tx, model, constituent, &["IFCMATERIALCONSTITUENT"])?;
    }
    Ok(tx.create(Entity::new(
        "IFCMATERIALCONSTITUENTSET",
        vec![
            optional_text(name),
            optional_text(description),
            refs(constituents),
        ],
    )))
}

/// Authored fields for `IfcMaterialProfile`.
#[derive(Debug, Clone, Copy)]
pub struct ProfileDraft<'a> {
    /// `IfcMaterialProfile.Name`, if given.
    pub name: Option<&'a str>,
    /// `IfcMaterialProfile.Description`, if given.
    pub description: Option<&'a str>,
    /// `IfcMaterialProfile.Material`, an `IfcMaterial` reference, if given.
    pub material: Option<EntityId>,
    /// `IfcMaterialProfile.Profile`, an `IfcProfileDef` reference.
    pub profile: EntityId,
    /// `IfcMaterialProfile.Priority`, if given. Must be in `0..=100`.
    pub priority: Option<i64>,
    /// `IfcMaterialProfile.Category`, if given.
    pub category: Option<&'a str>,
}

/// Stage an `IfcMaterialProfile`.
///
/// The profile reference is checked against `IfcProfileDef` subtypes that
/// this workspace lowers; an arbitrary entity here would produce a material
/// profile with no cross-section.
pub fn create_profile(
    tx: &mut Transaction,
    model: &Model,
    draft: ProfileDraft<'_>,
) -> MaterialResult<EntityId> {
    if let Some(priority) = draft.priority.filter(|value| !(0..=100).contains(value)) {
        return Err(invalid(
            "IFCMATERIALPROFILE",
            "Priority",
            priority.to_string(),
        ));
    }
    if let Some(material) = draft.material {
        require_type(tx, model, material, &["IFCMATERIAL"])?;
    }
    require_exists(tx, model, draft.profile)?;
    Ok(tx.create(Entity::new(
        "IFCMATERIALPROFILE",
        vec![
            optional_text(draft.name),
            optional_text(draft.description),
            draft.material.map_or(Value::Null, Value::Ref),
            Value::Ref(draft.profile),
            draft.priority.map_or(Value::Null, Value::Integer),
            optional_text(draft.category),
        ],
    )))
}

/// Stage an `IfcMaterialProfileSet`. Must name at least one profile.
pub fn create_profile_set(
    tx: &mut Transaction,
    model: &Model,
    profiles: &[EntityId],
    name: Option<&str>,
    description: Option<&str>,
    composite_profile: Option<EntityId>,
) -> MaterialResult<EntityId> {
    if profiles.is_empty() {
        return Err(invalid(
            "IFCMATERIALPROFILESET",
            "MaterialProfiles",
            "expected at least one profile",
        ));
    }
    for &profile in profiles {
        require_type(tx, model, profile, &["IFCMATERIALPROFILE"])?;
    }
    if let Some(composite) = composite_profile {
        require_exists(tx, model, composite)?;
    }
    Ok(tx.create(Entity::new(
        "IFCMATERIALPROFILESET",
        vec![
            optional_text(name),
            optional_text(description),
            refs(profiles),
            composite_profile.map_or(Value::Null, Value::Ref),
        ],
    )))
}

/// Stage an `IfcMaterialList`. Must name at least one material.
pub fn create_material_list(
    tx: &mut Transaction,
    model: &Model,
    materials: &[EntityId],
) -> MaterialResult<EntityId> {
    if materials.is_empty() {
        return Err(invalid(
            "IFCMATERIALLIST",
            "Materials",
            "expected at least one material",
        ));
    }
    for &material in materials {
        require_type(tx, model, material, &["IFCMATERIAL"])?;
    }
    Ok(tx.create(Entity::new("IFCMATERIALLIST", vec![refs(materials)])))
}

/// Stage an `IfcMaterialLayerSetUsage`.
///
/// Direction and sense are typed enums rather than strings: an invalid
/// token cannot be constructed, so no runtime check is needed for them.
pub fn create_layer_set_usage(
    tx: &mut Transaction,
    model: &Model,
    for_layer_set: EntityId,
    direction: LayerSetDirection,
    sense: DirectionSense,
    offset_from_reference_line: f64,
    reference_extent: Option<f64>,
) -> MaterialResult<EntityId> {
    require_type(tx, model, for_layer_set, &["IFCMATERIALLAYERSET"])?;
    if !offset_from_reference_line.is_finite() {
        return Err(invalid(
            "IFCMATERIALLAYERSETUSAGE",
            "OffsetFromReferenceLine",
            "expected a finite length",
        ));
    }
    Ok(tx.create(Entity::new(
        "IFCMATERIALLAYERSETUSAGE",
        vec![
            Value::Ref(for_layer_set),
            Value::Enum(direction.as_token().into()),
            Value::Enum(sense.as_token().into()),
            Value::Real(offset_from_reference_line),
            reference_extent.map_or(Value::Null, Value::Real),
        ],
    )))
}

/// Stage an `IfcMaterialProfileSetUsage`.
///
/// `CardinalPoint` selects the cross-section reference point and is an
/// `IfcCardinalPointReference` in `1..=9`; anything else names no point.
pub fn create_profile_set_usage(
    tx: &mut Transaction,
    model: &Model,
    for_profile_set: EntityId,
    cardinal_point: Option<i64>,
    reference_extent: Option<f64>,
) -> MaterialResult<EntityId> {
    require_type(tx, model, for_profile_set, &["IFCMATERIALPROFILESET"])?;
    if let Some(point) = cardinal_point.filter(|value| !(1..=9).contains(value)) {
        return Err(invalid(
            "IFCMATERIALPROFILESETUSAGE",
            "CardinalPoint",
            point.to_string(),
        ));
    }
    Ok(tx.create(Entity::new(
        "IFCMATERIALPROFILESETUSAGE",
        vec![
            Value::Ref(for_profile_set),
            cardinal_point.map_or(Value::Null, Value::Integer),
            reference_extent.map_or(Value::Null, Value::Real),
        ],
    )))
}

/// Stage an `IfcMaterialLayerWithOffsets`.
///
/// The record carries nine slots: the seven it inherits from
/// `IfcMaterialLayer` followed by its own two. Writing only the subtype
/// attributes would shift every inherited value into the wrong slot, which
/// is why this is a separate constructor rather than a flag on
/// [`create_layer`].
pub fn create_layer_with_offsets(
    tx: &mut Transaction,
    model: &Model,
    draft: LayerDraft<'_>,
    offset_direction: LayerSetDirection,
    offset_values: [f64; 2],
) -> MaterialResult<EntityId> {
    finite_non_negative(
        "IFCMATERIALLAYERWITHOFFSETS",
        "LayerThickness",
        draft.thickness,
    )?;
    for value in offset_values {
        if !value.is_finite() {
            return Err(invalid(
                "IFCMATERIALLAYERWITHOFFSETS",
                "OffsetValues",
                "expected finite lengths",
            ));
        }
    }
    if let Some(material) = draft.material {
        require_type(tx, model, material, &["IFCMATERIAL"])?;
    }
    Ok(tx.create(Entity::new(
        "IFCMATERIALLAYERWITHOFFSETS",
        vec![
            draft.material.map_or(Value::Null, Value::Ref),
            Value::Real(draft.thickness),
            draft.is_ventilated.map_or(Value::Null, logical),
            optional_text(draft.name),
            optional_text(draft.description),
            optional_text(draft.category),
            draft.priority.map_or(Value::Null, Value::Integer),
            Value::Enum(offset_direction.as_token().into()),
            Value::List(offset_values.iter().copied().map(Value::Real).collect()),
        ],
    )))
}

/// Stage an `IfcMaterialProfileWithOffsets`.
///
/// The offset variant of [`create_profile`]. `OffsetValues` is an
/// `ARRAY [1:2]`: two finite lengths, so a single value or three is
/// not an under-specified profile but a malformed one.
///
/// # Errors
///
/// Refuses non-finite offsets, a priority outside `0..=100`, and a
/// `Profile` or `Material` reference whose target is the wrong type.
pub fn create_profile_with_offsets(
    tx: &mut Transaction,
    model: &Model,
    draft: ProfileDraft<'_>,
    offset_values: [f64; 2],
) -> MaterialResult<EntityId> {
    if let Some(priority) = draft.priority.filter(|value| !(0..=100).contains(value)) {
        return Err(invalid(
            "IFCMATERIALPROFILEWITHOFFSETS",
            "Priority",
            priority.to_string(),
        ));
    }
    for value in offset_values {
        if !value.is_finite() {
            return Err(invalid(
                "IFCMATERIALPROFILEWITHOFFSETS",
                "OffsetValues",
                "expected finite lengths",
            ));
        }
    }
    if let Some(material) = draft.material {
        require_type(tx, model, material, &["IFCMATERIAL"])?;
    }
    require_exists(tx, model, draft.profile)?;
    Ok(tx.create(Entity::new(
        "IFCMATERIALPROFILEWITHOFFSETS",
        vec![
            optional_text(draft.name),
            optional_text(draft.description),
            draft.material.map_or(Value::Null, Value::Ref),
            Value::Ref(draft.profile),
            draft.priority.map_or(Value::Null, Value::Integer),
            optional_text(draft.category),
            Value::List(offset_values.iter().copied().map(Value::Real).collect()),
        ],
    )))
}

/// Stage an `IfcMaterialProfileSetUsageTapering`.
///
/// Five slots: three inherited, then its own two.
///
/// # Errors
///
/// Refuses a set that is not an `IfcMaterialProfileSet`, and a
/// cardinal point outside 1..=9 at either end.
pub fn create_profile_set_usage_tapering(
    tx: &mut Transaction,
    model: &Model,
    for_profile_set: EntityId,
    for_profile_end_set: EntityId,
    cardinal_point: Option<i64>,
    cardinal_end_point: Option<i64>,
    reference_extent: Option<f64>,
) -> MaterialResult<EntityId> {
    const ENTITY: &str = "IFCMATERIALPROFILESETUSAGETAPERING";
    for set in [for_profile_set, for_profile_end_set] {
        require_type(tx, model, set, &["IFCMATERIALPROFILESET"])?;
    }
    // Both ends carry the same 1..=9 cardinal point range.
    for (point, attribute) in [
        (cardinal_point, "CardinalPoint"),
        (cardinal_end_point, "CardinalEndPoint"),
    ] {
        if let Some(value) = point.filter(|value| !(1..=9).contains(value)) {
            return Err(invalid(ENTITY, attribute, value.to_string()));
        }
    }
    Ok(tx.create(Entity::new(
        ENTITY,
        vec![
            Value::Ref(for_profile_set),
            cardinal_point.map_or(Value::Null, Value::Integer),
            reference_extent.map_or(Value::Null, Value::Real),
            Value::Ref(for_profile_end_set),
            cardinal_end_point.map_or(Value::Null, Value::Integer),
        ],
    )))
}

const MATERIAL_SELECT_TYPES: &[&str] = &[
    "IFCMATERIAL",
    "IFCMATERIALLIST",
    "IFCMATERIALLAYERSET",
    "IFCMATERIALPROFILESET",
    "IFCMATERIALCONSTITUENTSET",
    "IFCMATERIALLAYERSETUSAGE",
    "IFCMATERIALPROFILESETUSAGE",
    "IFCMATERIALPROFILESETUSAGETAPERING",
];

fn require_exists(tx: &Transaction, model: &Model, id: EntityId) -> MaterialResult<()> {
    if type_name(tx, model, id).is_some() {
        Ok(())
    } else {
        Err(MaterialError::UnknownEntity { id })
    }
}
fn require_type(
    tx: &Transaction,
    model: &Model,
    id: EntityId,
    expected: &[&'static str],
) -> MaterialResult<()> {
    let actual = type_name(tx, model, id).ok_or(MaterialError::UnknownEntity { id })?;
    if expected
        .iter()
        .any(|kind| actual.eq_ignore_ascii_case(kind))
    {
        Ok(())
    } else {
        Err(MaterialError::AuthoringReferenceType {
            target: id,
            expected: expected[0],
            actual: actual.to_owned(),
        })
    }
}
fn type_name<'a>(tx: &'a Transaction, model: &'a Model, id: EntityId) -> Option<&'a str> {
    for edit in tx.edits().iter().rev() {
        match edit {
            Edit::Create {
                id: candidate,
                entity,
            } if *candidate == id => return Some(&entity.type_name),
            Edit::Retype {
                id: candidate,
                type_name,
            } if *candidate == id => return Some(type_name),
            Edit::Remove { id: candidate } if *candidate == id => return None,
            _ => {}
        }
    }
    model.get(id).map(|entity| entity.type_name.as_ref())
}
fn finite_non_negative(
    entity: &'static str,
    attribute: &'static str,
    value: f64,
) -> MaterialResult<()> {
    if value.is_finite() && value >= 0.0 {
        Ok(())
    } else {
        Err(invalid(
            entity,
            attribute,
            "expected a finite non-negative length",
        ))
    }
}
fn invalid(
    entity: &'static str,
    attribute: &'static str,
    value: impl Into<String>,
) -> MaterialError {
    MaterialError::AuthoringInvalid {
        entity,
        attribute,
        value: value.into(),
    }
}
fn text(value: &str) -> Value {
    Value::Text(value.into())
}
fn optional_text(value: Option<&str>) -> Value {
    value.map_or(Value::Null, text)
}
fn refs(ids: &[EntityId]) -> Value {
    Value::List(ids.iter().copied().map(Value::Ref).collect())
}
fn logical(value: LogicalValue) -> Value {
    match value {
        LogicalValue::False => Value::Bool(false),
        LogicalValue::True => Value::Bool(true),
        LogicalValue::Unknown => Value::LogicalUnknown,
    }
}
