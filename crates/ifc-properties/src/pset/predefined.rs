//! Predefined property sets: door and window lining and panel data,
//! permeable coverings, and the reinforcement/section families.
//!
//! These are `IfcPropertySetDefinition` subtypes, so they carry the four
//! `IfcRoot` slots before their own attributes. Unlike a generic
//! `IfcPropertySet` they have fixed, named, typed attributes and a set of
//! WHERE rules over them. The linings are in `lining.rs`; the release
//! each is written in is explained in `owned.rs`.

use ifc_model::guid::Guid;
use ifc_model::{Entity, EntityId, Model, Transaction, Value};

use crate::{PropertyError, PropertyResult};

use super::authoring::optional_text;
use super::owned::{stage_ifc4, stage_owned, Rooted};

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

/// Refuse a GlobalId that is not a 22-character IFC GUID.
pub(super) fn require_guid(entity: &'static str, global_id: &str) -> PropertyResult<()> {
    if Guid::parse(global_id).is_none() {
        return Err(invalid(entity, "GlobalId", global_id));
    }
    Ok(())
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

/// An `IfcNormalisedRatioMeasure`, bounded to `[0, 1]`.
pub(super) fn ratio(
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

pub(super) fn token(
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
/// Takes no model, so it writes the IFC4 layout with `OwnerHistory` `$`:
/// valid IFC4 and IFC4X3, never valid IFC2X3. Use
/// [`add_door_panel_properties_with_owner_history`] to write the model's
/// declared release.
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
    stage_ifc4(tx, door_panel(global_id, name, operation, position, panel)?)
}

/// [`add_door_panel_properties`] in the model's declared release, with a
/// caller-supplied `IfcOwnerHistory`, which IFC2X3 requires (#202).
///
/// # Errors
///
/// Those of [`add_door_panel_properties`], a token the release's own
/// enumeration does not list, and the release and owner-history refusals
/// of [`add_door_lining_properties_with_owner_history`](crate::add_door_lining_properties_with_owner_history).
/// Nothing is staged on an error.
#[allow(clippy::too_many_arguments)]
pub fn add_door_panel_properties_with_owner_history(
    tx: &mut Transaction,
    model: &Model,
    global_id: &str,
    name: Option<&str>,
    operation: &str,
    position: &str,
    panel: (Option<f64>, Option<f64>),
    owner_history: EntityId,
) -> PropertyResult<EntityId> {
    let rooted = door_panel(global_id, name, operation, position, panel)?;
    stage_owned(tx, model, rooted, owner_history)
}

fn door_panel<'a>(
    global_id: &'a str,
    name: Option<&'a str>,
    operation: &str,
    position: &str,
    (depth, width): (Option<f64>, Option<f64>),
) -> PropertyResult<Rooted<'a>> {
    const ENTITY: &str = "IFCDOORPANELPROPERTIES";
    require_guid(ENTITY, global_id)?;
    let values = vec![
        (
            "PanelDepth",
            measure(ENTITY, "PanelDepth", depth, Measure::Positive)?,
        ),
        (
            "PanelOperation",
            token(ENTITY, "PanelOperation", operation, DOOR_PANEL_OPERATION)?,
        ),
        ("PanelWidth", ratio(ENTITY, "PanelWidth", width)?),
        (
            "PanelPosition",
            token(ENTITY, "PanelPosition", position, DOOR_PANEL_POSITION)?,
        ),
    ];
    Ok(Rooted {
        entity: ENTITY,
        global_id,
        name,
        description: None,
        values,
    })
}

/// Stage an `IfcWindowPanelProperties`.
///
/// Takes no model, so it writes the IFC4 layout with `OwnerHistory` `$`:
/// valid IFC4 and IFC4X3, never valid IFC2X3. Use
/// [`add_window_panel_properties_with_owner_history`] to write the
/// model's declared release.
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
    let rooted = framed(WINDOW_PANEL, global_id, name, operation, position, frame)?;
    stage_ifc4(tx, rooted)
}

/// [`add_window_panel_properties`] in the model's declared release, with
/// a caller-supplied `IfcOwnerHistory`, which IFC2X3 requires (#202).
///
/// # Errors
///
/// Those of [`add_door_panel_properties_with_owner_history`], against the
/// window enumerations. Nothing is staged on an error.
#[allow(clippy::too_many_arguments)]
pub fn add_window_panel_properties_with_owner_history(
    tx: &mut Transaction,
    model: &Model,
    global_id: &str,
    name: Option<&str>,
    operation: &str,
    position: &str,
    frame: (Option<f64>, Option<f64>),
    owner_history: EntityId,
) -> PropertyResult<EntityId> {
    let rooted = framed(WINDOW_PANEL, global_id, name, operation, position, frame)?;
    stage_owned(tx, model, rooted, owner_history)
}

/// Stage an `IfcPermeableCoveringProperties`.
///
/// Takes no model, so it writes the IFC4 layout with `OwnerHistory` `$`:
/// valid IFC4 and IFC4X3, never valid IFC2X3. Use
/// [`add_permeable_covering_properties_with_owner_history`] to write the
/// model's declared release.
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
    let rooted = framed(PERMEABLE, global_id, name, operation, position, frame)?;
    stage_ifc4(tx, rooted)
}

/// [`add_permeable_covering_properties`] in the model's declared release,
/// with a caller-supplied `IfcOwnerHistory`, which IFC2X3 requires (#202).
///
/// # Errors
///
/// Those of [`add_door_panel_properties_with_owner_history`], against the
/// permeable-covering enumerations. Nothing is staged on an error.
#[allow(clippy::too_many_arguments)]
pub fn add_permeable_covering_properties_with_owner_history(
    tx: &mut Transaction,
    model: &Model,
    global_id: &str,
    name: Option<&str>,
    operation: &str,
    position: &str,
    frame: (Option<f64>, Option<f64>),
    owner_history: EntityId,
) -> PropertyResult<EntityId> {
    let rooted = framed(PERMEABLE, global_id, name, operation, position, frame)?;
    stage_owned(tx, model, rooted, owner_history)
}

/// A window panel or permeable covering: its entity and the enumerations
/// its `OperationType` and `PanelPosition` take.
struct Framed {
    entity: &'static str,
    operations: &'static [&'static str],
}

const WINDOW_PANEL: Framed = Framed {
    entity: "IFCWINDOWPANELPROPERTIES",
    operations: WINDOW_PANEL_OPERATION,
};
const PERMEABLE: Framed = Framed {
    entity: "IFCPERMEABLECOVERINGPROPERTIES",
    operations: PERMEABLE_COVERING_OPERATION,
};

fn framed<'a>(
    kind: Framed,
    global_id: &'a str,
    name: Option<&'a str>,
    operation: &str,
    position: &str,
    (frame_depth, frame_thickness): (Option<f64>, Option<f64>),
) -> PropertyResult<Rooted<'a>> {
    let entity = kind.entity;
    require_guid(entity, global_id)?;
    let values = vec![
        (
            "OperationType",
            token(entity, "OperationType", operation, kind.operations)?,
        ),
        (
            "PanelPosition",
            token(entity, "PanelPosition", position, WINDOW_PANEL_POSITION)?,
        ),
        (
            "FrameDepth",
            measure(entity, "FrameDepth", frame_depth, Measure::Positive)?,
        ),
        (
            "FrameThickness",
            measure(entity, "FrameThickness", frame_thickness, Measure::Positive)?,
        ),
    ];
    Ok(Rooted {
        entity,
        global_id,
        name,
        description: None,
        values,
    })
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
