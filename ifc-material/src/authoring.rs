//! Transactional IFC4 material authoring.
//!
//! These helpers only stage records. [`ifc_model::Transaction::commit`] owns
//! atomic graph/index application, so a failed batch cannot leave part of a
//! material graph in the model.
use std::collections::HashSet;

use ifc_model::{Edit, Entity, EntityId, Model, Transaction, Value};

use crate::{LogicalValue, MaterialError, MaterialResult};

/// Authored identity fields for `IfcMaterial`.
#[derive(Debug, Clone, Copy)]
pub struct MaterialDraft<'a> {
    pub name: &'a str,
    pub description: Option<&'a str>,
    pub category: Option<&'a str>,
}

/// Authored fields for `IfcMaterialLayer`.
#[derive(Debug, Clone, Copy)]
pub struct LayerDraft<'a> {
    pub material: Option<EntityId>,
    pub thickness: f64,
    pub is_ventilated: Option<LogicalValue>,
    pub name: Option<&'a str>,
    pub description: Option<&'a str>,
    pub category: Option<&'a str>,
    pub priority: Option<i64>,
}

/// Ordered composition fields for `IfcMaterialLayerSet`.
#[derive(Debug, Clone, Copy)]
pub struct LayerSetDraft<'a> {
    pub layers: &'a [EntityId],
    pub name: Option<&'a str>,
    pub description: Option<&'a str>,
}

/// Authored fields for `IfcRelAssociatesMaterial`.
#[derive(Debug, Clone, Copy)]
pub struct MaterialAssignmentDraft<'a> {
    pub global_id: &'a str,
    pub name: Option<&'a str>,
    pub description: Option<&'a str>,
    pub related_objects: &'a [EntityId],
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
