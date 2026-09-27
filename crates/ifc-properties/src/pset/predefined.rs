//! Predefined property sets: door and window lining and panel data,
//! permeable coverings, and the reinforcement/section families.
//!
//! These are `IfcPropertySetDefinition` subtypes, so they carry the four
//! `IfcRoot` slots before their own attributes. Unlike a generic
//! `IfcPropertySet` they have fixed, named, typed attributes and a set of
//! WHERE rules over them.
//!
//! # Paired attributes
//!
//! The lining rules are pairing constraints, and door and window state
//! them differently. A door's transom and casing pairs are XOR: both or
//! neither. A window's transom and mullion offsets are ordered: the
//! second may only appear when the first does. Writing one half of
//! either pair produces a file that parses and reports a dimension
//! nothing can interpret.

use ifc_model::guid::Guid;
use ifc_model::{Entity, EntityId, Transaction, Value};

use crate::{PropertyError, PropertyResult};

use super::authoring::optional_text;

pub(super) fn invalid(
    entity: &'static str,
    attribute: &'static str,
    value: impl Into<String>,
) -> PropertyError {
    PropertyError::AuthoringInvalid {
        entity,
        attribute,
        value: value.into(),
    }
}

pub(super) fn root_slots(
    entity: &'static str,
    global_id: &str,
    name: Option<&str>,
    description: Option<&str>,
    arity: usize,
) -> PropertyResult<Vec<Value>> {
    if Guid::parse(global_id).is_none() {
        return Err(invalid(entity, "GlobalId", global_id));
    }
    let mut attributes = vec![Value::Null; arity];
    attributes[0] = Value::Text(global_id.into());
    attributes[2] = optional_text(name);
    attributes[3] = optional_text(description);
    Ok(attributes)
}

/// The three length measure kinds these entities use.
///
/// `IfcLengthMeasure` admits negatives: an offset may sit either side of
/// its reference. Collapsing all three into a non-negative check would
/// silently refuse legal files.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Measure {
    /// `IfcPositiveLengthMeasure`: strictly greater than zero.
    Positive,
    /// `IfcNonNegativeLengthMeasure`: zero or greater.
    NonNegative,
    /// `IfcLengthMeasure`: any finite value, including negative.
    Length,
}

pub(super) fn measure(
    entity: &'static str,
    attribute: &'static str,
    value: Option<f64>,
    kind: Measure,
) -> PropertyResult<Value> {
    let Some(value) = value else {
        return Ok(Value::Null);
    };
    let ok = value.is_finite()
        && match kind {
            Measure::Positive => value > 0.0,
            Measure::NonNegative => value >= 0.0,
            Measure::Length => true,
        };
    if !ok {
        return Err(invalid(entity, attribute, format!("{value}")));
    }
    Ok(Value::Real(value))
}

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

    let mut attributes = root_slots(ENTITY, global_id, draft.name, draft.description, 17)?;
    attributes[4] = measure(ENTITY, "LiningDepth", draft.lining_depth, Measure::Positive)?;
    attributes[5] = measure(
        ENTITY,
        "LiningThickness",
        draft.lining_thickness,
        Measure::NonNegative,
    )?;
    attributes[6] = measure(
        ENTITY,
        "ThresholdDepth",
        draft.threshold_depth,
        Measure::Positive,
    )?;
    attributes[7] = measure(
        ENTITY,
        "ThresholdThickness",
        draft.threshold_thickness,
        Measure::NonNegative,
    )?;
    attributes[8] = measure(
        ENTITY,
        "TransomThickness",
        draft.transom_thickness,
        Measure::NonNegative,
    )?;
    attributes[9] = measure(
        ENTITY,
        "TransomOffset",
        draft.transom_offset,
        Measure::Length,
    )?;
    attributes[10] = measure(ENTITY, "LiningOffset", draft.lining_offset, Measure::Length)?;
    attributes[11] = measure(
        ENTITY,
        "ThresholdOffset",
        draft.threshold_offset,
        Measure::Length,
    )?;
    attributes[12] = measure(
        ENTITY,
        "CasingThickness",
        draft.casing_thickness,
        Measure::Positive,
    )?;
    attributes[13] = measure(ENTITY, "CasingDepth", draft.casing_depth, Measure::Positive)?;
    attributes[14] = draft.shape_aspect_style.map_or(Value::Null, Value::Ref);
    attributes[15] = measure(
        ENTITY,
        "LiningToPanelOffsetX",
        draft.lining_to_panel_offset_x,
        Measure::Length,
    )?;
    attributes[16] = measure(
        ENTITY,
        "LiningToPanelOffsetY",
        draft.lining_to_panel_offset_y,
        Measure::Length,
    )?;
    Ok(tx.create(Entity::new(ENTITY, attributes)))
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

fn ratio(
    entity: &'static str,
    attribute: &'static str,
    value: Option<f64>,
) -> PropertyResult<Value> {
    let Some(value) = value else {
        return Ok(Value::Null);
    };
    if !value.is_finite() || !(0.0..=1.0).contains(&value) {
        return Err(invalid(entity, attribute, format!("{value}")));
    }
    Ok(Value::Real(value))
}

/// Stage an `IfcWindowLiningProperties`.
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

    let mut attributes = root_slots(ENTITY, global_id, draft.name, draft.description, 16)?;
    attributes[4] = measure(ENTITY, "LiningDepth", draft.lining_depth, Measure::Positive)?;
    attributes[5] = measure(
        ENTITY,
        "LiningThickness",
        draft.lining_thickness,
        Measure::NonNegative,
    )?;
    attributes[6] = measure(
        ENTITY,
        "TransomThickness",
        draft.transom_thickness,
        Measure::NonNegative,
    )?;
    attributes[7] = measure(
        ENTITY,
        "MullionThickness",
        draft.mullion_thickness,
        Measure::NonNegative,
    )?;
    attributes[8] = ratio(ENTITY, "FirstTransomOffset", draft.first_transom_offset)?;
    attributes[9] = ratio(ENTITY, "SecondTransomOffset", draft.second_transom_offset)?;
    attributes[10] = ratio(ENTITY, "FirstMullionOffset", draft.first_mullion_offset)?;
    attributes[11] = ratio(ENTITY, "SecondMullionOffset", draft.second_mullion_offset)?;
    attributes[12] = draft.shape_aspect_style.map_or(Value::Null, Value::Ref);
    attributes[13] = measure(ENTITY, "LiningOffset", draft.lining_offset, Measure::Length)?;
    attributes[14] = measure(
        ENTITY,
        "LiningToPanelOffsetX",
        draft.lining_to_panel_offset_x,
        Measure::Length,
    )?;
    attributes[15] = measure(
        ENTITY,
        "LiningToPanelOffsetY",
        draft.lining_to_panel_offset_y,
        Measure::Length,
    )?;
    Ok(tx.create(Entity::new(ENTITY, attributes)))
}

const DOOR_PANEL_OPERATION: &[&str] = &[
    "DOUBLE_ACTING",
    "FIXEDPANEL",
    "FOLDING",
    "REVOLVING",
    "ROLLINGUP",
    "SLIDING",
    "SWINGING",
    "USERDEFINED",
    "NOTDEFINED",
];
const DOOR_PANEL_POSITION: &[&str] = &["LEFT", "MIDDLE", "RIGHT", "NOTDEFINED"];
const WINDOW_PANEL_OPERATION: &[&str] = &[
    "BOTTOMHUNG",
    "FIXEDCASEMENT",
    "OTHEROPERATION",
    "PIVOTHORIZONTAL",
    "PIVOTVERTICAL",
    "REMOVABLECASEMENT",
    "SIDEHUNGLEFTHAND",
    "SIDEHUNGRIGHTHAND",
    "SLIDINGHORIZONTAL",
    "SLIDINGVERTICAL",
    "TILTANDTURNLEFTHAND",
    "TILTANDTURNRIGHTHAND",
    "TOPHUNG",
    "NOTDEFINED",
];
const WINDOW_PANEL_POSITION: &[&str] = &["BOTTOM", "LEFT", "MIDDLE", "RIGHT", "TOP", "NOTDEFINED"];
const PERMEABLE_COVERING_OPERATION: &[&str] =
    &["GRILL", "LOUVER", "SCREEN", "USERDEFINED", "NOTDEFINED"];

fn token(
    entity: &'static str,
    attribute: &'static str,
    value: &str,
    allowed: &[&str],
) -> PropertyResult<Value> {
    if !allowed.contains(&value) {
        return Err(invalid(entity, attribute, value));
    }
    Ok(Value::Enum(value.into()))
}

/// Stage an `IfcDoorPanelProperties`.
///
/// `PanelOperation` and `PanelPosition` are mandatory: the schema does
/// not mark them OPTIONAL, so a panel always states how it moves and
/// where it sits.
///
/// # Errors
///
/// Refuses a malformed GlobalId, a token outside its own enumeration,
/// a non-positive panel depth, and a panel width outside `[0, 1]`
/// (`PanelWidth` is an `IfcNormalisedRatioMeasure`, a fraction of the
/// opening rather than a length).
pub fn add_door_panel_properties(
    tx: &mut Transaction,
    global_id: &str,
    name: Option<&str>,
    operation: &str,
    position: &str,
    panel: (Option<f64>, Option<f64>),
) -> PropertyResult<EntityId> {
    const ENTITY: &str = "IFCDOORPANELPROPERTIES";
    let (depth, width) = panel;
    let mut attributes = root_slots(ENTITY, global_id, name, None, 9)?;
    attributes[4] = measure(ENTITY, "PanelDepth", depth, Measure::Positive)?;
    attributes[5] = token(ENTITY, "PanelOperation", operation, DOOR_PANEL_OPERATION)?;
    attributes[6] = ratio(ENTITY, "PanelWidth", width)?;
    attributes[7] = token(ENTITY, "PanelPosition", position, DOOR_PANEL_POSITION)?;
    Ok(tx.create(Entity::new(ENTITY, attributes)))
}

/// Stage an `IfcWindowPanelProperties`.
///
/// # Errors
///
/// As for the door panel, against the window enumerations. Note that
/// `IfcWindowPanelOperationEnum` has no `USERDEFINED` member, so a
/// caller cannot fall back to it here.
pub fn add_window_panel_properties(
    tx: &mut Transaction,
    global_id: &str,
    name: Option<&str>,
    operation: &str,
    position: &str,
    frame: (Option<f64>, Option<f64>),
) -> PropertyResult<EntityId> {
    const ENTITY: &str = "IFCWINDOWPANELPROPERTIES";
    let (frame_depth, frame_thickness) = frame;
    let mut attributes = root_slots(ENTITY, global_id, name, None, 9)?;
    attributes[4] = token(ENTITY, "OperationType", operation, WINDOW_PANEL_OPERATION)?;
    attributes[5] = token(ENTITY, "PanelPosition", position, WINDOW_PANEL_POSITION)?;
    attributes[6] = measure(ENTITY, "FrameDepth", frame_depth, Measure::Positive)?;
    attributes[7] = measure(ENTITY, "FrameThickness", frame_thickness, Measure::Positive)?;
    Ok(tx.create(Entity::new(ENTITY, attributes)))
}

/// Stage an `IfcPermeableCoveringProperties`.
///
/// # Errors
///
/// As for the panels, against the permeable-covering operation
/// enumeration and the window panel positions it shares.
pub fn add_permeable_covering_properties(
    tx: &mut Transaction,
    global_id: &str,
    name: Option<&str>,
    operation: &str,
    position: &str,
    frame: (Option<f64>, Option<f64>),
) -> PropertyResult<EntityId> {
    const ENTITY: &str = "IFCPERMEABLECOVERINGPROPERTIES";
    let (frame_depth, frame_thickness) = frame;
    let mut attributes = root_slots(ENTITY, global_id, name, None, 9)?;
    attributes[4] = token(
        ENTITY,
        "OperationType",
        operation,
        PERMEABLE_COVERING_OPERATION,
    )?;
    attributes[5] = token(ENTITY, "PanelPosition", position, WINDOW_PANEL_POSITION)?;
    attributes[6] = measure(ENTITY, "FrameDepth", frame_depth, Measure::Positive)?;
    attributes[7] = measure(ENTITY, "FrameThickness", frame_thickness, Measure::Positive)?;
    Ok(tx.create(Entity::new(ENTITY, attributes)))
}

/// The STEP type name of an `IfcValue`, for the homogeneity rule.
///
/// TYPEOF compares the declared measure, so a wrapped value is named by
/// its wrapper and a bare literal by its syntactic kind. Two values that
/// print the same can still differ here, which is exactly what WR01 is
/// about.
fn value_type(value: &Value) -> Option<String> {
    Some(match value {
        Value::Typed { type_name, .. } => type_name.to_ascii_uppercase(),
        Value::Bool(_) | Value::LogicalUnknown => "BOOLEAN".to_owned(),
        Value::Integer(_) => "INTEGER".to_owned(),
        Value::Real(_) => "REAL".to_owned(),
        Value::Text(_) => "STRING".to_owned(),
        Value::Binary(_) => "BINARY".to_owned(),
        Value::Enum(_) => "ENUM".to_owned(),
        // `$`, `*`, refs and aggregates are not single measures.
        _ => return None,
    })
}

/// Stage an `IfcPropertyEnumeration`.
///
/// # Errors
///
/// Refuses a blank name (UR1 makes it the unique key), an empty value
/// list, a value that is not a single measure, a duplicate value (the
/// list is UNIQUE), and a list mixing measure types (WR01).
pub fn add_property_enumeration(
    tx: &mut Transaction,
    name: &str,
    values: Vec<Value>,
    unit: Option<EntityId>,
) -> PropertyResult<EntityId> {
    const ENTITY: &str = "IFCPROPERTYENUMERATION";
    if name.trim().is_empty() {
        return Err(invalid(ENTITY, "Name", name));
    }
    if values.is_empty() {
        return Err(invalid(ENTITY, "EnumerationValues", "empty"));
    }
    let mut first: Option<String> = None;
    for value in &values {
        let Some(kind) = value_type(value) else {
            return Err(invalid(ENTITY, "EnumerationValues", format!("{value:?}")));
        };
        match &first {
            None => first = Some(kind),
            Some(expected) if *expected != kind => {
                return Err(invalid(
                    ENTITY,
                    "EnumerationValues",
                    format!("WR01: {kind} among {expected}"),
                ));
            }
            Some(_) => {}
        }
    }
    for (index, value) in values.iter().enumerate() {
        if values[..index].contains(value) {
            return Err(invalid(ENTITY, "EnumerationValues", "duplicate value"));
        }
    }
    let attributes = vec![
        Value::Text(name.into()),
        Value::List(values),
        unit.map_or(Value::Null, Value::Ref),
    ];
    Ok(tx.create(Entity::new(ENTITY, attributes)))
}

const COMPLEX_TEMPLATE_TYPE: &[&str] = &["P_COMPLEX", "Q_COMPLEX"];

/// Stage an `IfcComplexPropertyTemplate`.
///
/// # Errors
///
/// Refuses a malformed GlobalId, an empty template set (the attribute
/// is `SET [1:?]` when present), a duplicate template reference, and a
/// `TemplateType` outside its enumeration.
///
/// `NoSelfReference` needs no check: [`Transaction::create`] allocates
/// the id as it stages the entity, so a caller cannot hold that id in
/// order to pass it as one of its own children.
///
/// `UniquePropertyNames` is stated over the templates' names, which a
/// staged entity cannot be read back to supply. Callers pass
/// `(name, id)` pairs, matching `add_property_set`.
pub fn add_complex_property_template(
    tx: &mut Transaction,
    global_id: &str,
    name: Option<&str>,
    usage: (Option<&str>, Option<&str>),
    templates: &[(&str, EntityId)],
) -> PropertyResult<EntityId> {
    const ENTITY: &str = "IFCCOMPLEXPROPERTYTEMPLATE";
    let (usage_name, template_type) = usage;
    let mut attributes = root_slots(ENTITY, global_id, name, None, 7)?;
    attributes[4] = optional_text(usage_name);
    if let Some(kind) = template_type {
        attributes[5] = token(ENTITY, "TemplateType", kind, COMPLEX_TEMPLATE_TYPE)?;
    }
    if !templates.is_empty() {
        for (index, (child, _)) in templates.iter().enumerate() {
            if templates[..index].iter().any(|(seen, _)| seen == child) {
                return Err(invalid(ENTITY, "HasPropertyTemplates", (*child).to_owned()));
            }
        }
        attributes[6] = Value::List(templates.iter().map(|(_, id)| Value::Ref(*id)).collect());
    }
    Ok(tx.create(Entity::new(ENTITY, attributes)))
}

/// Stage an `IfcPropertyDependencyRelationship`.
///
/// # Errors
///
/// Refuses a relationship whose depending and dependant property are
/// the same entity (NoSelfReference): a property cannot derive from
/// itself, and such a record makes a dependency walk loop forever.
pub fn add_property_dependency_relationship(
    tx: &mut Transaction,
    name: Option<&str>,
    description: Option<&str>,
    properties: (EntityId, EntityId),
    expression: Option<&str>,
) -> PropertyResult<EntityId> {
    const ENTITY: &str = "IFCPROPERTYDEPENDENCYRELATIONSHIP";
    let (depending, dependant) = properties;
    if depending == dependant {
        return Err(invalid(ENTITY, "DependingProperty", "NoSelfReference"));
    }
    let attributes = vec![
        optional_text(name),
        optional_text(description),
        Value::Ref(depending),
        Value::Ref(dependant),
        optional_text(expression),
    ];
    Ok(tx.create(Entity::new(ENTITY, attributes)))
}
