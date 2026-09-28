//! Reading a window's operation inputs exactly from the declared release.
//!
//! Every attribute is found by name in the bound release's table, never by a
//! restated slot: `IfcWindow.PartitioningType` exists only from IFC4 on, and
//! IFC2X3 states the partitioning as `IfcWindowStyle.OperationType`, where
//! IFC4 and IFC4X3 name it `IfcWindowType.PartitioningType`. Relationship
//! structure is validated first by `exact_predefined_sets`, which refuses
//! malformed assignments, several type objects, and relationship subtypes
//! relating the window.

use std::sync::Arc;

use ifc_geometry::Transform;
use ifc_model::{Entity, EntityId, Model};
use ifc_properties::{
    exact_predefined_sets, exact_schema, exact_unit, ExactPredefinedSet, ExactPropertyError,
    ExactSource, ExactValue, SchemaVersion,
};
use ifc_schema::Schema;

use super::WindowOperationError as E;
use super::{
    layout, WindowOperationError, WindowPanelOperation, WindowPanelPosition, WindowPartitioning,
};
use crate::operation;

/// Everything the derivation needs, read and validated.
pub(super) struct WindowInputs {
    pub(super) partitioning: WindowPartitioning,
    pub(super) partitioning_source: ExactSource,
    pub(super) panel_source: ExactSource,
    /// Metres.
    pub(super) overall_width: f64,
    /// Metres.
    pub(super) overall_height: f64,
    /// World transform in metres.
    pub(super) world: Transform,
    pub(super) panels: Vec<Panel>,
    /// The lining set, read only when the partitioning splits the window.
    pub(super) lining: Option<Lining>,
}

/// One `IfcWindowPanelProperties`, as the layout needs it.
#[derive(Debug, Clone)]
pub(super) struct Panel {
    pub(super) set: EntityId,
    pub(super) position: WindowPanelPosition,
    pub(super) operation: WindowPanelOperation,
    /// `FrameDepth` in metres, `None` for `$`.
    pub(super) frame_depth: Option<f64>,
    /// `FrameThickness` in metres, `None` for `$`.
    pub(super) frame_thickness: Option<f64>,
}

/// The governing `IfcWindowLiningProperties`' split ratios, `None` for `$`.
#[derive(Debug, Clone)]
pub(super) struct Lining {
    pub(super) source: ExactSource,
    pub(super) set: EntityId,
    pub(super) first_mullion: Option<f64>,
    pub(super) second_mullion: Option<f64>,
    pub(super) first_transom: Option<f64>,
    pub(super) second_transom: Option<f64>,
}

pub(super) fn read_window(
    model: &Model,
    window: EntityId,
) -> Result<WindowInputs, WindowOperationError> {
    let version = exact_schema(model)?;
    let schema = ifc_schema::for_version(version).expect("exact releases are bundled");
    let entity = model
        .get(window)
        .ok_or(ExactPropertyError::MissingReference {
            from: window,
            to: window,
        })?;
    // Validates the window, every property and type relationship in the
    // file, and the type object, before anything below trusts their
    // structure.
    let sets = exact_predefined_sets(model, window, "IfcWindowPanelProperties")?;
    if !schema.is_a(&entity.type_name, "IFCWINDOW") {
        return Err(E::NotAWindow {
            entity: window,
            type_name: entity.type_name.clone(),
        });
    }
    let (written, partitioning_source) = partitioning(model, schema, version, window, entity)?;
    let partitioning = layout::classify_partitioning(&written)?;
    let length = |name, missing| {
        operation::positive_length(model, schema, window, entity, name)
            .map_err(E::read)?
            .ok_or(missing)
    };
    let overall_width = length("OverallWidth", E::MissingOverallWidth { window })?;
    let overall_height = length("OverallHeight", E::MissingOverallHeight { window })?;
    let (panel_source, panels) =
        operation::governing(&sets).ok_or(E::NoPanelProperties { window })?;
    let panels = panels
        .iter()
        .map(|set| panel(model, set))
        .collect::<Result<_, _>>()?;
    let lining = if partitioning == WindowPartitioning::SinglePanel {
        None
    } else {
        lining(model, window)?
    };
    let world = operation::world_transform(model, window).map_err(E::read)?;
    Ok(WindowInputs {
        partitioning,
        partitioning_source,
        panel_source,
        overall_width,
        overall_height,
        world,
        panels,
        lining,
    })
}

/// The partitioning and where it was stated.
///
/// IFC4 `IfcWindow.PartitioningType`: "shall only be used, if no type
/// object IfcWindowType is assigned, providing its own
/// IfcWindowType.PartitioningType". A value on only one of them governs;
/// two values must agree, since the specification leaves no rule to prefer
/// either. IFC2X3 declares no occurrence attribute, and its
/// `IfcWindowStyle` states the partitioning as `OperationType`.
fn partitioning(
    model: &Model,
    schema: &'static Schema,
    version: SchemaVersion,
    window: EntityId,
    entity: &Entity,
) -> Result<(Arc<str>, ExactSource), WindowOperationError> {
    let occurrence =
        operation::enumeration(schema, window, entity, "PartitioningType").map_err(E::read)?;
    let typed = match operation::type_object(model, schema, window).map_err(E::read)? {
        Some(type_id) => {
            let type_entity = model.get(type_id).expect("validated reference");
            // IFC4 `CorrectStyleAssigned` and IFC4X3 `CorrectTypeAssigned`
            // require an `IfcWindowType`; IFC2X3 has only `IfcWindowStyle`.
            let (expected, attribute) = match version {
                SchemaVersion::Ifc2x3 => ("IFCWINDOWSTYLE", "OperationType"),
                SchemaVersion::Ifc4 | SchemaVersion::Ifc4x3 => {
                    ("IFCWINDOWTYPE", "PartitioningType")
                }
                // `exact_schema` admits no other release; a later one is
                // refused here too rather than read as IFC4.
                other => {
                    return Err(ExactPropertyError::UnsupportedSchema {
                        schema: format!("{other:?}"),
                    }
                    .into())
                }
            };
            if !schema.is_a(&type_entity.type_name, expected) {
                return Err(E::UnsupportedTypeObject {
                    type_object: type_id,
                    type_name: type_entity.type_name.clone(),
                });
            }
            // Required in every release, so `$` is malformed.
            let value = operation::enumeration(schema, type_id, type_entity, attribute)
                .map_err(E::read)?
                .ok_or(E::MalformedAttribute {
                    entity: type_id,
                    attribute,
                })?;
            Some((type_id, value))
        }
        None => None,
    };
    match (occurrence, typed) {
        (Some(occurrence), Some((type_object, type_value))) if occurrence != type_value => {
            Err(E::ConflictingPartitioningType {
                occurrence,
                type_object,
                type_value,
            })
        }
        (Some(occurrence), _) => Ok((occurrence, ExactSource::Occurrence)),
        (None, Some((type_object, value))) => Ok((value, ExactSource::Type(type_object))),
        (None, None) => Err(E::MissingPartitioningType { window }),
    }
}

/// One panel set's position, operation and frame dimensions.
fn panel(model: &Model, set: &ExactPredefinedSet) -> Result<Panel, WindowOperationError> {
    let malformed = |attribute| E::MalformedAttribute {
        entity: set.set_id,
        attribute,
    };
    let text = |name: &'static str| match set.attribute(name).map(|a| &a.value) {
        Some(ExactValue::Enum(value)) => Ok(value.clone()),
        _ => Err(malformed(name)),
    };
    let position = match &*text("PanelPosition")? {
        "LEFT" => WindowPanelPosition::Left,
        "MIDDLE" => WindowPanelPosition::Middle,
        "RIGHT" => WindowPanelPosition::Right,
        "BOTTOM" => WindowPanelPosition::Bottom,
        "TOP" => WindowPanelPosition::Top,
        "NOTDEFINED" => WindowPanelPosition::NotDefined,
        _ => return Err(malformed("PanelPosition")),
    };
    let operation = layout::classify_panel(set.set_id, &text("OperationType")?)?;
    let length = |name: &'static str| -> Result<Option<f64>, WindowOperationError> {
        let Some(property) = set.attribute(name) else {
            return Err(malformed(name));
        };
        match property.value {
            ExactValue::Null => Ok(None),
            // `IfcPositiveLengthMeasure`: a finite length greater than zero.
            ExactValue::Real(value) if value.is_finite() && value > 0.0 => {
                let unit = property.value_type.as_deref().ok_or(malformed(name))?;
                Ok(Some(
                    value * exact_unit(model, unit, property.unit_id)?.scale,
                ))
            }
            _ => Err(malformed(name)),
        }
    };
    Ok(Panel {
        set: set.set_id,
        position,
        operation,
        frame_depth: length("FrameDepth")?,
        frame_thickness: length("FrameThickness")?,
    })
}

/// The one `IfcWindowLiningProperties` of the governing source, or `None`
/// when neither the occurrence nor its type carries one.
fn lining(model: &Model, window: EntityId) -> Result<Option<Lining>, WindowOperationError> {
    let sets = exact_predefined_sets(model, window, "IfcWindowLiningProperties")?;
    let Some((source, governing)) = operation::governing(&sets) else {
        return Ok(None);
    };
    let [set] = governing[..] else {
        return Err(E::LiningCount {
            found: governing.len(),
        });
    };
    let ratio = |name: &'static str| match set.attribute(name).map(|a| &a.value) {
        Some(ExactValue::Null) => Ok(None),
        Some(ExactValue::Real(value)) => Ok(Some(*value)),
        _ => Err(E::MalformedAttribute {
            entity: set.set_id,
            attribute: name,
        }),
    };
    Ok(Some(Lining {
        source,
        set: set.set_id,
        first_mullion: ratio("FirstMullionOffset")?,
        second_mullion: ratio("SecondMullionOffset")?,
        first_transom: ratio("FirstTransomOffset")?,
        second_transom: ratio("SecondTransomOffset")?,
    }))
}
