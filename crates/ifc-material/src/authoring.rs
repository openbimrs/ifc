//! Transactional material authoring, bound to the model's release.
//!
//! These helpers only stage records. [`ifc_model::Transaction::commit`] owns
//! atomic graph/index application, so a failed batch cannot leave part of a
//! material graph in the model.
//!
//! Every record is laid out from the bundled table of the release the
//! model's header declares (see [`crate::material_schema`]), never from IFC4
//! by assumption. A record type the release lacks (a constituent or profile
//! set for IFC2X3) is refused with [`MaterialError::EntityNotInSchema`]; a
//! draft value for an attribute it lacks (an IFC2X3 layer `Name`) with
//! [`MaterialError::AuthoringNotInSchema`]; and an attribute it requires
//! that the call leaves unset (the IFC2X3 `OwnerHistory`) with
//! [`MaterialError::AuthoringRequired`]. Nothing is staged on refusal.
mod composites;
mod relationships;

pub use composites::{
    create_constituent, create_constituent_set, create_profile, create_profile_set,
    create_profile_set_usage, create_profile_set_usage_tapering, create_profile_with_offsets,
    ConstituentDraft, ProfileDraft,
};
pub use relationships::{
    create_material_classification_relationship, create_material_definition_representation,
    create_material_properties, create_material_relationship,
};

use std::collections::HashSet;

use ifc_model::{Edit, EntityId, Model, Transaction, Value};

use crate::release::Release;
use crate::{DirectionSense, LayerSetDirection, LogicalValue, MaterialError, MaterialResult};

/// Authored identity fields for `IfcMaterial`.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct MaterialDraft<'a> {
    /// `IfcMaterial.Name`.
    pub name: &'a str,
    /// `IfcMaterial.Description`, if given. IFC4 onwards.
    pub description: Option<&'a str>,
    /// `IfcMaterial.Category`, if given. IFC4 onwards.
    pub category: Option<&'a str>,
}

impl<'a> MaterialDraft<'a> {
    /// Starts a draft for a material called `name`.
    #[must_use]
    pub const fn new(name: &'a str) -> Self {
        Self {
            name,
            description: None,
            category: None,
        }
    }

    /// Sets `Description`.
    #[must_use]
    pub const fn description(mut self, value: &'a str) -> Self {
        self.description = Some(value);
        self
    }

    /// Sets `Category`.
    #[must_use]
    pub const fn category(mut self, value: &'a str) -> Self {
        self.category = Some(value);
        self
    }
}

/// Authored fields for `IfcMaterialLayer`.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct LayerDraft<'a> {
    /// `IfcMaterialLayer.Material`, an `IfcMaterial` reference, if given.
    pub material: Option<EntityId>,
    /// `IfcMaterialLayer.LayerThickness`. Must be finite and non-negative;
    /// strictly positive for IFC2X3.
    pub thickness: f64,
    /// `IfcMaterialLayer.IsVentilated`, if given.
    pub is_ventilated: Option<LogicalValue>,
    /// `IfcMaterialLayer.Name`, if given. IFC4 onwards.
    pub name: Option<&'a str>,
    /// `IfcMaterialLayer.Description`, if given. IFC4 onwards.
    pub description: Option<&'a str>,
    /// `IfcMaterialLayer.Category`, if given. IFC4 onwards.
    pub category: Option<&'a str>,
    /// `IfcMaterialLayer.Priority`, if given. Must be in `0..=100`. IFC4
    /// onwards.
    pub priority: Option<i64>,
}

impl<'a> LayerDraft<'a> {
    /// Starts a draft for a layer `thickness` thick.
    #[must_use]
    pub const fn new(thickness: f64) -> Self {
        Self {
            material: None,
            thickness,
            is_ventilated: None,
            name: None,
            description: None,
            category: None,
            priority: None,
        }
    }

    /// Sets `Material`, an `IfcMaterial` reference.
    #[must_use]
    pub const fn material(mut self, value: EntityId) -> Self {
        self.material = Some(value);
        self
    }

    /// Sets `IsVentilated`.
    #[must_use]
    pub const fn is_ventilated(mut self, value: LogicalValue) -> Self {
        self.is_ventilated = Some(value);
        self
    }

    /// Sets `Name`.
    #[must_use]
    pub const fn name(mut self, value: &'a str) -> Self {
        self.name = Some(value);
        self
    }

    /// Sets `Description`.
    #[must_use]
    pub const fn description(mut self, value: &'a str) -> Self {
        self.description = Some(value);
        self
    }

    /// Sets `Category`.
    #[must_use]
    pub const fn category(mut self, value: &'a str) -> Self {
        self.category = Some(value);
        self
    }

    /// Sets `Priority`.
    #[must_use]
    pub const fn priority(mut self, value: i64) -> Self {
        self.priority = Some(value);
        self
    }
}

/// Ordered composition fields for `IfcMaterialLayerSet`.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct LayerSetDraft<'a> {
    /// `IfcMaterialLayerSet.MaterialLayers`, in set order. Must be non-empty.
    pub layers: &'a [EntityId],
    /// `IfcMaterialLayerSet.LayerSetName`, if given.
    pub name: Option<&'a str>,
    /// `IfcMaterialLayerSet.Description`, if given. IFC4 onwards.
    pub description: Option<&'a str>,
}

impl<'a> LayerSetDraft<'a> {
    /// Starts a draft for a set of `layers`, in set order.
    #[must_use]
    pub const fn new(layers: &'a [EntityId]) -> Self {
        Self {
            layers,
            name: None,
            description: None,
        }
    }

    /// Sets `LayerSetName`.
    #[must_use]
    pub const fn name(mut self, value: &'a str) -> Self {
        self.name = Some(value);
        self
    }

    /// Sets `Description`.
    #[must_use]
    pub const fn description(mut self, value: &'a str) -> Self {
        self.description = Some(value);
        self
    }
}

/// Authored fields for `IfcRelAssociatesMaterial`.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
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

impl<'a> MaterialAssignmentDraft<'a> {
    /// Starts a draft associating `relating_material` with
    /// `related_objects`.
    #[must_use]
    pub const fn new(
        global_id: &'a str,
        related_objects: &'a [EntityId],
        relating_material: EntityId,
    ) -> Self {
        Self {
            global_id,
            name: None,
            description: None,
            related_objects,
            relating_material,
        }
    }

    /// Sets `Name`.
    #[must_use]
    pub const fn name(mut self, value: &'a str) -> Self {
        self.name = Some(value);
        self
    }

    /// Sets `Description`.
    #[must_use]
    pub const fn description(mut self, value: &'a str) -> Self {
        self.description = Some(value);
        self
    }
}

/// Stage a material identity record in the model's release layout.
///
/// # Errors
///
/// [`MaterialError::AuthoringNotInSchema`] for a `description` or
/// `category` in an IFC2X3 model, whose `IfcMaterial` declares `Name` only,
/// and the release-binding errors.
pub fn create_material(
    tx: &mut Transaction,
    model: &Model,
    draft: MaterialDraft<'_>,
) -> MaterialResult<EntityId> {
    let record = Release::of(model).record(
        "IFCMATERIAL",
        vec![
            ("Name", text(draft.name)),
            ("Description", optional_text(draft.description)),
            ("Category", optional_text(draft.category)),
        ],
    )?;
    Ok(tx.create(record))
}

/// The layer attributes [`create_layer`] and [`create_layer_with_offsets`]
/// share, after checking the thickness against the release's measure.
fn layer_values(
    release: Release<'_>,
    entity: &'static str,
    draft: &LayerDraft<'_>,
) -> MaterialResult<Vec<(&'static str, Value)>> {
    let declared = release.declared(entity, "LayerThickness")?;
    let positive = declared
        .type_name
        .eq_ignore_ascii_case("IfcPositiveLengthMeasure");
    if !draft.thickness.is_finite() || draft.thickness < 0.0 {
        return Err(invalid(
            entity,
            "LayerThickness",
            "expected a finite non-negative length",
        ));
    }
    if positive && draft.thickness == 0.0 {
        return Err(invalid(
            entity,
            "LayerThickness",
            "expected a positive length (IfcPositiveLengthMeasure)",
        ));
    }
    if let Some(priority) = draft.priority.filter(|value| !(0..=100).contains(value)) {
        return Err(invalid(entity, "Priority", priority.to_string()));
    }
    Ok(vec![
        ("Material", draft.material.map_or(Value::Null, Value::Ref)),
        ("LayerThickness", Value::Real(draft.thickness)),
        (
            "IsVentilated",
            draft.is_ventilated.map_or(Value::Null, logical),
        ),
        ("Name", optional_text(draft.name)),
        ("Description", optional_text(draft.description)),
        ("Category", optional_text(draft.category)),
        (
            "Priority",
            draft.priority.map_or(Value::Null, Value::Integer),
        ),
    ])
}

/// Stage a layer after checking its scalar and material-reference invariants.
///
/// # Errors
///
/// Refuses a non-finite or negative thickness (zero too for IFC2X3), a
/// priority outside `0..=100`, a material that is not an `IfcMaterial`, and
/// for IFC2X3 any `name`, `description`, `category` or `priority`
/// ([`MaterialError::AuthoringNotInSchema`]).
pub fn create_layer(
    tx: &mut Transaction,
    model: &Model,
    draft: LayerDraft<'_>,
) -> MaterialResult<EntityId> {
    const ENTITY: &str = "IFCMATERIALLAYER";
    let release = Release::of(model);
    let values = layer_values(release, ENTITY, &draft)?;
    if let Some(material) = draft.material {
        require_type(tx, model, release, material, &["IFCMATERIAL"])?;
    }
    Ok(tx.create(release.record(ENTITY, values)?))
}

/// Stage a non-empty ordered layer set. Layers may have been created earlier
/// in this transaction; their staged type is checked just like stored records.
///
/// # Errors
///
/// Refuses an empty layer list, a member that is not a layer of the model's
/// release, and a `description` for IFC2X3.
pub fn create_layer_set(
    tx: &mut Transaction,
    model: &Model,
    draft: LayerSetDraft<'_>,
) -> MaterialResult<EntityId> {
    const ENTITY: &str = "IFCMATERIALLAYERSET";
    let release = Release::of(model);
    release.require_entity(ENTITY, None)?;
    if draft.layers.is_empty() {
        return Err(invalid(
            ENTITY,
            "MaterialLayers",
            "expected at least one layer",
        ));
    }
    for &layer in draft.layers {
        require_type(
            tx,
            model,
            release,
            layer,
            &["IFCMATERIALLAYER", "IFCMATERIALLAYERWITHOFFSETS"],
        )?;
    }
    let record = release.record(
        ENTITY,
        vec![
            ("MaterialLayers", refs(draft.layers)),
            ("LayerSetName", optional_text(draft.name)),
            ("Description", optional_text(draft.description)),
        ],
    )?;
    Ok(tx.create(record))
}

/// Stage a product/type material association after validating the IFC GlobalId,
/// non-empty relation end, and `IfcMaterialSelect` branch.
///
/// `OwnerHistory` is left unset. IFC2X3 requires it, so an IFC2X3 model is
/// refused with [`MaterialError::AuthoringRequired`]; use
/// [`associate_material_with_owner_history`] there.
///
/// # Errors
///
/// Refuses a malformed GlobalId, an empty or duplicated `RelatedObjects`,
/// and a `RelatingMaterial` that is no `IfcMaterialSelect` member of the
/// model's release.
pub fn associate_material(
    tx: &mut Transaction,
    model: &Model,
    draft: MaterialAssignmentDraft<'_>,
) -> MaterialResult<EntityId> {
    associate(tx, model, draft, None)
}

/// [`associate_material`] with a caller-supplied `IfcOwnerHistory`, which
/// IFC2X3 requires. The owner history is never invented here (build one with
/// `ifc-author`).
///
/// # Errors
///
/// Those of [`associate_material`], and an `owner_history` that is not an
/// `IfcOwnerHistory`.
pub fn associate_material_with_owner_history(
    tx: &mut Transaction,
    model: &Model,
    draft: MaterialAssignmentDraft<'_>,
    owner_history: EntityId,
) -> MaterialResult<EntityId> {
    associate(tx, model, draft, Some(owner_history))
}

fn associate(
    tx: &mut Transaction,
    model: &Model,
    draft: MaterialAssignmentDraft<'_>,
    owner_history: Option<EntityId>,
) -> MaterialResult<EntityId> {
    const ENTITY: &str = "IFCRELASSOCIATESMATERIAL";
    let release = Release::of(model);
    release.require_entity(ENTITY, None)?;
    if ifc_model::guid::Guid::parse(draft.global_id).is_none() {
        return Err(invalid(ENTITY, "GlobalId", "expected IFC compressed GUID"));
    }
    if draft.related_objects.is_empty() {
        return Err(invalid(
            ENTITY,
            "RelatedObjects",
            "expected at least one object",
        ));
    }
    let mut unique = HashSet::new();
    for &object in draft.related_objects {
        if !unique.insert(object) {
            return Err(invalid(
                ENTITY,
                "RelatedObjects",
                "duplicate object reference",
            ));
        }
        require_exists(tx, model, object)?;
    }
    require_accepts(
        tx,
        model,
        release,
        ENTITY,
        "RelatingMaterial",
        draft.relating_material,
    )?;
    if let Some(owner_history) = owner_history {
        require_type(tx, model, release, owner_history, &["IFCOWNERHISTORY"])?;
    }
    let record = release.record(
        ENTITY,
        vec![
            ("GlobalId", text(draft.global_id)),
            (
                "OwnerHistory",
                owner_history.map_or(Value::Null, Value::Ref),
            ),
            ("Name", optional_text(draft.name)),
            ("Description", optional_text(draft.description)),
            ("RelatedObjects", refs(draft.related_objects)),
            ("RelatingMaterial", Value::Ref(draft.relating_material)),
        ],
    )?;
    Ok(tx.create(record))
}

/// Stage an `IfcMaterialList`. Must name at least one material.
pub fn create_material_list(
    tx: &mut Transaction,
    model: &Model,
    materials: &[EntityId],
) -> MaterialResult<EntityId> {
    const ENTITY: &str = "IFCMATERIALLIST";
    let release = Release::of(model);
    release.require_entity(ENTITY, None)?;
    if materials.is_empty() {
        return Err(invalid(
            ENTITY,
            "Materials",
            "expected at least one material",
        ));
    }
    for &material in materials {
        require_type(tx, model, release, material, &["IFCMATERIAL"])?;
    }
    Ok(tx.create(release.record(ENTITY, vec![("Materials", refs(materials))])?))
}

/// Stage an `IfcMaterialLayerSetUsage`.
///
/// Direction and sense are typed enums rather than strings: an invalid
/// token cannot be constructed, so no runtime check is needed for them.
///
/// # Errors
///
/// Refuses a set that is not an `IfcMaterialLayerSet`, a non-finite offset,
/// and a `reference_extent` for IFC2X3, which has no `ReferenceExtent`.
pub fn create_layer_set_usage(
    tx: &mut Transaction,
    model: &Model,
    for_layer_set: EntityId,
    direction: LayerSetDirection,
    sense: DirectionSense,
    offset_from_reference_line: f64,
    reference_extent: Option<f64>,
) -> MaterialResult<EntityId> {
    const ENTITY: &str = "IFCMATERIALLAYERSETUSAGE";
    let release = Release::of(model);
    release.require_entity(ENTITY, None)?;
    require_type(tx, model, release, for_layer_set, &["IFCMATERIALLAYERSET"])?;
    if !offset_from_reference_line.is_finite() {
        return Err(invalid(
            ENTITY,
            "OffsetFromReferenceLine",
            "expected a finite length",
        ));
    }
    let record = release.record(
        ENTITY,
        vec![
            ("ForLayerSet", Value::Ref(for_layer_set)),
            (
                "LayerSetDirection",
                Value::Enum(direction.as_token().into()),
            ),
            ("DirectionSense", Value::Enum(sense.as_token().into())),
            (
                "OffsetFromReferenceLine",
                Value::Real(offset_from_reference_line),
            ),
            (
                "ReferenceExtent",
                reference_extent.map_or(Value::Null, Value::Real),
            ),
        ],
    )?;
    Ok(tx.create(record))
}

/// Stage an `IfcMaterialLayerWithOffsets`. IFC4 onwards.
///
/// The record carries nine slots: the seven it inherits from
/// `IfcMaterialLayer` followed by its own two. Writing only the subtype
/// attributes would shift every inherited value into the wrong slot, which
/// is why this is a separate constructor rather than a flag on
/// [`create_layer`].
///
/// # Errors
///
/// Those of [`create_layer`], non-finite offsets, and
/// [`MaterialError::EntityNotInSchema`] for IFC2X3.
pub fn create_layer_with_offsets(
    tx: &mut Transaction,
    model: &Model,
    draft: LayerDraft<'_>,
    offset_direction: LayerSetDirection,
    offset_values: [f64; 2],
) -> MaterialResult<EntityId> {
    const ENTITY: &str = "IFCMATERIALLAYERWITHOFFSETS";
    let release = Release::of(model);
    let mut values = layer_values(release, ENTITY, &draft)?;
    if offset_values.iter().any(|value| !value.is_finite()) {
        return Err(invalid(ENTITY, "OffsetValues", "expected finite lengths"));
    }
    if let Some(material) = draft.material {
        require_type(tx, model, release, material, &["IFCMATERIAL"])?;
    }
    values.push((
        "OffsetDirection",
        Value::Enum(offset_direction.as_token().into()),
    ));
    values.push(("OffsetValues", reals(&offset_values)));
    Ok(tx.create(release.record(ENTITY, values)?))
}

/// Fail unless `id` exists, staged or committed.
pub(crate) fn require_exists(tx: &Transaction, model: &Model, id: EntityId) -> MaterialResult<()> {
    if type_name(tx, model, id).is_some() {
        Ok(())
    } else {
        Err(MaterialError::UnknownEntity { id })
    }
}

/// Fail unless `id` has one of `expected`'s types that the model's release
/// can instantiate: an `IfcMaterialLayerWithOffsets` is no IFC2X3 layer.
pub(crate) fn require_type(
    tx: &Transaction,
    model: &Model,
    release: Release<'_>,
    id: EntityId,
    expected: &[&'static str],
) -> MaterialResult<()> {
    let actual = type_name(tx, model, id).ok_or(MaterialError::UnknownEntity { id })?;
    if expected
        .iter()
        .any(|kind| actual.eq_ignore_ascii_case(kind) && release.instantiates(kind))
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

/// Fail unless `id`'s type is a legal value of `entity.attribute` in the
/// model's release, as its bundled table declares it (inheritance and
/// SELECT membership included).
pub(crate) fn require_accepts(
    tx: &Transaction,
    model: &Model,
    release: Release<'_>,
    entity: &'static str,
    attribute: &'static str,
    id: EntityId,
) -> MaterialResult<()> {
    let actual = type_name(tx, model, id).ok_or(MaterialError::UnknownEntity { id })?;
    let (accepted, declared) = release.accepts(entity, attribute, actual)?;
    if accepted {
        Ok(())
    } else {
        Err(MaterialError::AuthoringReferenceType {
            target: id,
            expected: declared,
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

pub(crate) fn invalid(
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

pub(crate) fn optional_text(value: Option<&str>) -> Value {
    value.map_or(Value::Null, text)
}

pub(crate) fn refs(ids: &[EntityId]) -> Value {
    Value::List(ids.iter().copied().map(Value::Ref).collect())
}

pub(crate) fn reals(values: &[f64]) -> Value {
    Value::List(values.iter().copied().map(Value::Real).collect())
}

fn logical(value: LogicalValue) -> Value {
    match value {
        LogicalValue::False => Value::Bool(false),
        LogicalValue::True => Value::Bool(true),
        LogicalValue::Unknown => Value::LogicalUnknown,
    }
}
