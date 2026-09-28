//! Door and window lining properties.
//!
//! # Paired attributes
//!
//! The lining rules are pairing constraints, and door and window state
//! them differently. A door's transom and casing pairs are XOR: both or
//! neither. A window's transom and mullion offsets are ordered: the
//! second may only appear when the first does. Writing one half of
//! either pair produces a file that parses and reports a dimension
//! nothing can interpret.
//!
//! # Release
//!
//! [`add_door_lining_properties`] and [`add_window_lining_properties`]
//! take no model and write the IFC4 layout; the `*_with_owner_history`
//! variants write the model's declared release (see `owned.rs`). IFC2X3
//! declares neither `LiningToPanelOffsetX/Y` nor, on a window,
//! `LiningOffset`, and types the thicknesses as positive lengths.

use ifc_model::{EntityId, Model, Transaction, Value};

use crate::PropertyResult;

use super::owned::{stage_ifc4, stage_owned, Rooted};
use super::predefined::{invalid, measure, ratio, require_guid, Measure};

/// Attributes of an `IfcDoorLiningProperties`.
#[derive(Debug, Clone, Copy, Default)]
pub struct DoorLiningDraft<'a> {
    /// `Name`.
    pub name: Option<&'a str>,
    /// `Description`.
    pub description: Option<&'a str>,
    /// `LiningDepth`, a positive length. Requires `lining_thickness` (WR31).
    pub lining_depth: Option<f64>,
    /// `LiningThickness`, a non-negative length.
    pub lining_thickness: Option<f64>,
    /// `ThresholdDepth`, a positive length. Requires `threshold_thickness` (WR32).
    pub threshold_depth: Option<f64>,
    /// `ThresholdThickness`, a non-negative length.
    pub threshold_thickness: Option<f64>,
    /// `TransomThickness`, a non-negative length. Paired with `transom_offset` (WR33).
    pub transom_thickness: Option<f64>,
    /// `TransomOffset`, a length. Paired with `transom_thickness` (WR33).
    pub transom_offset: Option<f64>,
    /// `LiningOffset`, a length.
    pub lining_offset: Option<f64>,
    /// `ThresholdOffset`, a length.
    pub threshold_offset: Option<f64>,
    /// `CasingThickness`, a positive length. Paired with `casing_depth` (WR34).
    pub casing_thickness: Option<f64>,
    /// `CasingDepth`, a positive length. Paired with `casing_thickness` (WR34).
    pub casing_depth: Option<f64>,
    /// `ShapeAspectStyle`.
    pub shape_aspect_style: Option<EntityId>,
    /// `LiningToPanelOffsetX`, a length.
    pub lining_to_panel_offset_x: Option<f64>,
    /// `LiningToPanelOffsetY`, a length.
    pub lining_to_panel_offset_y: Option<f64>,
}

/// Stage an `IfcDoorLiningProperties`.
///
/// Takes no model, so it writes the IFC4 layout with `OwnerHistory` `$`:
/// valid IFC4 and IFC4X3, never valid IFC2X3. Use
/// [`add_door_lining_properties_with_owner_history`] to write the model's
/// declared release.
///
/// # Errors
///
/// Refuses a malformed GlobalId; a depth without its thickness (WR31,
/// WR32); a transom or casing pair with exactly one half set (WR33,
/// WR34); and any measure that violates its schema measure type.
///
/// WR35 requires the set to define an `IfcDoorType`. That is a property
/// of the attachment, not of this record, so it is enforced where the
/// set is attached rather than invented here.
pub fn add_door_lining_properties(
    tx: &mut Transaction,
    global_id: &str,
    draft: DoorLiningDraft<'_>,
) -> PropertyResult<EntityId> {
    stage_ifc4(tx, door_lining(global_id, draft)?)
}

/// [`add_door_lining_properties`] in the model's declared release, with a
/// caller-supplied `IfcOwnerHistory`, which IFC2X3 requires (#202).
///
/// # Errors
///
/// Those of [`add_door_lining_properties`], and: a model that binds no
/// single known release; an `owner_history` that is not in the model or
/// staged ([`MissingEntity`](crate::PropertyError::MissingEntity)) or is
/// not an `IfcOwnerHistory`
/// ([`AuthoringInvalid`](crate::PropertyError::AuthoringInvalid)); in
/// IFC2X3 a `lining_to_panel_offset_x/y`
/// ([`AuthoringNotInSchema`](crate::PropertyError::AuthoringNotInSchema))
/// and a zero thickness, which IFC2X3 types as `IfcPositiveLengthMeasure`.
/// Nothing is staged on an error.
pub fn add_door_lining_properties_with_owner_history(
    tx: &mut Transaction,
    model: &Model,
    global_id: &str,
    draft: DoorLiningDraft<'_>,
    owner_history: EntityId,
) -> PropertyResult<EntityId> {
    stage_owned(tx, model, door_lining(global_id, draft)?, owner_history)
}

fn door_lining<'a>(global_id: &'a str, draft: DoorLiningDraft<'a>) -> PropertyResult<Rooted<'a>> {
    const ENTITY: &str = "IFCDOORLININGPROPERTIES";
    if draft.lining_depth.is_some() && draft.lining_thickness.is_none() {
        return Err(invalid(
            ENTITY,
            "LiningThickness",
            "WR31: a lining depth needs its thickness",
        ));
    }
    if draft.threshold_depth.is_some() && draft.threshold_thickness.is_none() {
        return Err(invalid(
            ENTITY,
            "ThresholdThickness",
            "WR32: a threshold depth needs its thickness",
        ));
    }
    if draft.transom_offset.is_some() != draft.transom_thickness.is_some() {
        return Err(invalid(
            ENTITY,
            "TransomOffset",
            "WR33: transom offset and thickness are all or nothing",
        ));
    }
    if draft.casing_depth.is_some() != draft.casing_thickness.is_some() {
        return Err(invalid(
            ENTITY,
            "CasingDepth",
            "WR34: casing depth and thickness are all or nothing",
        ));
    }
    require_guid(ENTITY, global_id)?;
    let m = |attribute, value, kind| measure(ENTITY, attribute, value, kind);
    let values = vec![
        (
            "LiningDepth",
            m("LiningDepth", draft.lining_depth, Measure::Positive)?,
        ),
        (
            "LiningThickness",
            m(
                "LiningThickness",
                draft.lining_thickness,
                Measure::NonNegative,
            )?,
        ),
        (
            "ThresholdDepth",
            m("ThresholdDepth", draft.threshold_depth, Measure::Positive)?,
        ),
        (
            "ThresholdThickness",
            m(
                "ThresholdThickness",
                draft.threshold_thickness,
                Measure::NonNegative,
            )?,
        ),
        (
            "TransomThickness",
            m(
                "TransomThickness",
                draft.transom_thickness,
                Measure::NonNegative,
            )?,
        ),
        (
            "TransomOffset",
            m("TransomOffset", draft.transom_offset, Measure::Length)?,
        ),
        (
            "LiningOffset",
            m("LiningOffset", draft.lining_offset, Measure::Length)?,
        ),
        (
            "ThresholdOffset",
            m("ThresholdOffset", draft.threshold_offset, Measure::Length)?,
        ),
        (
            "CasingThickness",
            m("CasingThickness", draft.casing_thickness, Measure::Positive)?,
        ),
        (
            "CasingDepth",
            m("CasingDepth", draft.casing_depth, Measure::Positive)?,
        ),
        (
            "ShapeAspectStyle",
            draft.shape_aspect_style.map_or(Value::Null, Value::Ref),
        ),
        (
            "LiningToPanelOffsetX",
            m(
                "LiningToPanelOffsetX",
                draft.lining_to_panel_offset_x,
                Measure::Length,
            )?,
        ),
        (
            "LiningToPanelOffsetY",
            m(
                "LiningToPanelOffsetY",
                draft.lining_to_panel_offset_y,
                Measure::Length,
            )?,
        ),
    ];
    Ok(Rooted {
        entity: ENTITY,
        global_id,
        name: draft.name,
        description: draft.description,
        values,
    })
}

/// Attributes of an `IfcWindowLiningProperties`.
#[derive(Debug, Clone, Copy, Default)]
pub struct WindowLiningDraft<'a> {
    /// `Name`.
    pub name: Option<&'a str>,
    /// `Description`.
    pub description: Option<&'a str>,
    /// `LiningDepth`, a positive length. Requires `lining_thickness` (WR31).
    pub lining_depth: Option<f64>,
    /// `LiningThickness`, a non-negative length.
    pub lining_thickness: Option<f64>,
    /// `TransomThickness`, a non-negative length.
    pub transom_thickness: Option<f64>,
    /// `MullionThickness`, a non-negative length.
    pub mullion_thickness: Option<f64>,
    /// `FirstTransomOffset`, a normalised ratio.
    pub first_transom_offset: Option<f64>,
    /// `SecondTransomOffset`, a normalised ratio. Needs the first (WR32).
    pub second_transom_offset: Option<f64>,
    /// `FirstMullionOffset`, a normalised ratio.
    pub first_mullion_offset: Option<f64>,
    /// `SecondMullionOffset`, a normalised ratio. Needs the first (WR33).
    pub second_mullion_offset: Option<f64>,
    /// `ShapeAspectStyle`.
    pub shape_aspect_style: Option<EntityId>,
    /// `LiningOffset`, a length.
    pub lining_offset: Option<f64>,
    /// `LiningToPanelOffsetX`, a length.
    pub lining_to_panel_offset_x: Option<f64>,
    /// `LiningToPanelOffsetY`, a length.
    pub lining_to_panel_offset_y: Option<f64>,
}

/// Stage an `IfcWindowLiningProperties`.
///
/// Takes no model, so it writes the IFC4 layout with `OwnerHistory` `$`:
/// valid IFC4 and IFC4X3, never valid IFC2X3. Use
/// [`add_window_lining_properties_with_owner_history`] to write the
/// model's declared release.
///
/// # Errors
///
/// Refuses a malformed GlobalId; a lining depth without its thickness
/// (WR31); a second transom or mullion offset without the first (WR32,
/// WR33); and any measure outside its schema type. The offsets are
/// `IfcNormalisedRatioMeasure`, so they are bounded to `[0, 1]` rather
/// than treated as free lengths.
///
/// Unlike the door rules, these are ordered rather than XOR: a first
/// offset alone is legal, a second alone is not.
pub fn add_window_lining_properties(
    tx: &mut Transaction,
    global_id: &str,
    draft: WindowLiningDraft<'_>,
) -> PropertyResult<EntityId> {
    stage_ifc4(tx, window_lining(global_id, draft)?)
}

/// [`add_window_lining_properties`] in the model's declared release, with
/// a caller-supplied `IfcOwnerHistory`, which IFC2X3 requires (#202).
///
/// # Errors
///
/// Those of [`add_window_lining_properties`] and the release and
/// owner-history refusals of
/// [`add_door_lining_properties_with_owner_history`]. IFC2X3 declares no
/// `LiningOffset` and no `LiningToPanelOffsetX/Y` on a window lining.
/// Nothing is staged on an error.
pub fn add_window_lining_properties_with_owner_history(
    tx: &mut Transaction,
    model: &Model,
    global_id: &str,
    draft: WindowLiningDraft<'_>,
    owner_history: EntityId,
) -> PropertyResult<EntityId> {
    stage_owned(tx, model, window_lining(global_id, draft)?, owner_history)
}

fn window_lining<'a>(
    global_id: &'a str,
    draft: WindowLiningDraft<'a>,
) -> PropertyResult<Rooted<'a>> {
    const ENTITY: &str = "IFCWINDOWLININGPROPERTIES";
    if draft.lining_depth.is_some() && draft.lining_thickness.is_none() {
        return Err(invalid(
            ENTITY,
            "LiningThickness",
            "WR31: a lining depth needs its thickness",
        ));
    }
    if draft.second_transom_offset.is_some() && draft.first_transom_offset.is_none() {
        return Err(invalid(
            ENTITY,
            "SecondTransomOffset",
            "WR32: a second transom offset needs the first",
        ));
    }
    if draft.second_mullion_offset.is_some() && draft.first_mullion_offset.is_none() {
        return Err(invalid(
            ENTITY,
            "SecondMullionOffset",
            "WR33: a second mullion offset needs the first",
        ));
    }
    require_guid(ENTITY, global_id)?;
    let m = |attribute, value, kind| measure(ENTITY, attribute, value, kind);
    let r = |attribute, value| ratio(ENTITY, attribute, value);
    let values = vec![
        (
            "LiningDepth",
            m("LiningDepth", draft.lining_depth, Measure::Positive)?,
        ),
        (
            "LiningThickness",
            m(
                "LiningThickness",
                draft.lining_thickness,
                Measure::NonNegative,
            )?,
        ),
        (
            "TransomThickness",
            m(
                "TransomThickness",
                draft.transom_thickness,
                Measure::NonNegative,
            )?,
        ),
        (
            "MullionThickness",
            m(
                "MullionThickness",
                draft.mullion_thickness,
                Measure::NonNegative,
            )?,
        ),
        (
            "FirstTransomOffset",
            r("FirstTransomOffset", draft.first_transom_offset)?,
        ),
        (
            "SecondTransomOffset",
            r("SecondTransomOffset", draft.second_transom_offset)?,
        ),
        (
            "FirstMullionOffset",
            r("FirstMullionOffset", draft.first_mullion_offset)?,
        ),
        (
            "SecondMullionOffset",
            r("SecondMullionOffset", draft.second_mullion_offset)?,
        ),
        (
            "ShapeAspectStyle",
            draft.shape_aspect_style.map_or(Value::Null, Value::Ref),
        ),
        (
            "LiningOffset",
            m("LiningOffset", draft.lining_offset, Measure::Length)?,
        ),
        (
            "LiningToPanelOffsetX",
            m(
                "LiningToPanelOffsetX",
                draft.lining_to_panel_offset_x,
                Measure::Length,
            )?,
        ),
        (
            "LiningToPanelOffsetY",
            m(
                "LiningToPanelOffsetY",
                draft.lining_to_panel_offset_y,
                Measure::Length,
            )?,
        ),
    ];
    Ok(Rooted {
        entity: ENTITY,
        global_id,
        name: draft.name,
        description: draft.description,
        values,
    })
}
