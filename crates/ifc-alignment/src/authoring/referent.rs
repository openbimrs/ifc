//! Referents and the linear placements that position them.
//!
//! A referent marks a station along an alignment. Positioning it needs
//! three entities: a point expressed as a distance along the basis
//! curve, a linear axis placement at that point, and the placement
//! itself. They are authored separately because each is referenced on
//! its own elsewhere in a file.

use ifc_model::{Entity, EntityId, Transaction, Value};

use super::{finite, guid, invalid};
use crate::error::AlignmentError;
use crate::slot;

/// Stage an `IfcCartesianPoint`.
///
/// # Errors
///
/// Refuses coordinates outside `LIST [1:3]` and any non-finite value.
pub fn cartesian_point(
    tx: &mut Transaction,
    coordinates: &[f64],
) -> Result<EntityId, AlignmentError> {
    if coordinates.is_empty() || coordinates.len() > 3 {
        return Err(invalid(
            "IFCCARTESIANPOINT",
            "Coordinates",
            format!("{} coordinates", coordinates.len()),
        ));
    }
    for value in coordinates {
        finite("IFCCARTESIANPOINT", "Coordinates", *value)?;
    }
    Ok(tx.create(Entity::new(
        "IFCCARTESIANPOINT",
        vec![Value::List(
            coordinates.iter().copied().map(Value::Real).collect(),
        )],
    )))
}

/// Stage an `IfcPointByDistanceExpression`.
///
/// A position stated as a distance along a curve rather than as
/// coordinates, which is what keeps a referent attached to the
/// alignment when the geometry is re-fitted.
///
/// # Errors
///
/// Refuses a non-finite distance or offset.
pub fn point_by_distance(
    tx: &mut Transaction,
    distance_along: f64,
    offsets: (Option<f64>, Option<f64>, Option<f64>),
    basis_curve: EntityId,
) -> Result<EntityId, AlignmentError> {
    finite(
        "IFCPOINTBYDISTANCEEXPRESSION",
        "DistanceAlong",
        distance_along,
    )?;
    let (lateral, vertical, longitudinal) = offsets;
    for (name, value) in [
        ("OffsetLateral", lateral),
        ("OffsetVertical", vertical),
        ("OffsetLongitudinal", longitudinal),
    ] {
        if let Some(value) = value {
            finite("IFCPOINTBYDISTANCEEXPRESSION", name, value)?;
        }
    }
    let mut attrs = vec![Value::Null; slot::point_by_distance::ARITY];
    attrs[slot::point_by_distance::DISTANCE_ALONG] = Value::Real(distance_along);
    attrs[slot::point_by_distance::OFFSET_LATERAL] = lateral.map_or(Value::Null, Value::Real);
    attrs[slot::point_by_distance::OFFSET_VERTICAL] = vertical.map_or(Value::Null, Value::Real);
    attrs[slot::point_by_distance::OFFSET_LONGITUDINAL] =
        longitudinal.map_or(Value::Null, Value::Real);
    attrs[slot::point_by_distance::BASIS_CURVE] = Value::Ref(basis_curve);
    Ok(tx.create(Entity::new("IFCPOINTBYDISTANCEEXPRESSION", attrs)))
}

/// Stage an `IfcAxis2PlacementLinear`.
///
/// # Errors
///
/// Never fails; the signature matches its siblings so callers can
/// chain the three placement entities with one error type.
pub fn axis2_placement_linear(
    tx: &mut Transaction,
    location: EntityId,
    axis: Option<EntityId>,
    ref_direction: Option<EntityId>,
) -> Result<EntityId, AlignmentError> {
    let mut attrs = vec![Value::Null; slot::axis2_placement_linear::ARITY];
    attrs[slot::axis2_placement_linear::LOCATION] = Value::Ref(location);
    attrs[slot::axis2_placement_linear::AXIS] = axis.map_or(Value::Null, Value::Ref);
    attrs[slot::axis2_placement_linear::REF_DIRECTION] =
        ref_direction.map_or(Value::Null, Value::Ref);
    Ok(tx.create(Entity::new("IFCAXIS2PLACEMENTLINEAR", attrs)))
}

/// Stage an `IfcLinearPlacement`.
///
/// `CartesianPosition` is optional and deliberately left to the
/// caller: it caches the resolved world position, and this crate will
/// not compute one, since deriving coordinates from an alignment curve
/// belongs to the geometry kernel.
///
/// # Errors
///
/// Never fails; kept fallible for symmetry with its siblings.
pub fn linear_placement(
    tx: &mut Transaction,
    relative_placement: EntityId,
    placement_rel_to: Option<EntityId>,
    cartesian_position: Option<EntityId>,
) -> Result<EntityId, AlignmentError> {
    let mut attrs = vec![Value::Null; slot::linear_placement::ARITY];
    attrs[slot::linear_placement::PLACEMENT_REL_TO] =
        placement_rel_to.map_or(Value::Null, Value::Ref);
    attrs[slot::linear_placement::RELATIVE_PLACEMENT] = Value::Ref(relative_placement);
    attrs[slot::linear_placement::CARTESIAN_POSITION] =
        cartesian_position.map_or(Value::Null, Value::Ref);
    Ok(tx.create(Entity::new("IFCLINEARPLACEMENT", attrs)))
}

/// Stage an `IfcReferent`.
///
/// # Errors
///
/// Refuses a `GlobalId` that is not 22 characters.
pub fn referent(
    tx: &mut Transaction,
    global_id: &str,
    name: Option<&str>,
    predefined_type: Option<&str>,
    placement: Option<EntityId>,
) -> Result<EntityId, AlignmentError> {
    let mut attrs = vec![Value::Null; slot::referent::ARITY];
    attrs[slot::product::GLOBAL_ID] = guid("IFCREFERENT", global_id)?;
    if let Some(name) = name {
        attrs[slot::product::NAME] = Value::Text(name.into());
    }
    attrs[slot::product::OBJECT_PLACEMENT] = placement.map_or(Value::Null, Value::Ref);
    attrs[slot::referent::PREDEFINED_TYPE] =
        predefined_type.map_or(Value::Null, |token| Value::Enum(token.into()));
    Ok(tx.create(Entity::new("IFCREFERENT", attrs)))
}

/// Stage the `Pset_Stationing` property set for a referent.
///
/// Stationing is carried by a property set, not by an attribute, so a
/// referent without one is positioned but has no station. The reader
/// looks for this exact set name and these exact property names,
/// which is why authoring them is a named helper rather than a
/// generic property-set call: a typo here produces a referent the
/// stationing reader silently skips.
///
/// Returns the `IfcRelDefinesByProperties` that binds the set to the
/// referent.
///
/// # Errors
///
/// Refuses a non-finite station and a `GlobalId` that is not 22
/// characters.
pub fn stationing(
    tx: &mut Transaction,
    pset_global_id: &str,
    rel_global_id: &str,
    referent: EntityId,
    station: f64,
    incoming_station: Option<f64>,
    has_increasing_station: Option<bool>,
) -> Result<EntityId, AlignmentError> {
    finite("IFCPROPERTYSET", "Station", station)?;
    if let Some(value) = incoming_station {
        finite("IFCPROPERTYSET", "IncomingStation", value)?;
    }
    let pset_guid = guid("IFCPROPERTYSET", pset_global_id)?;
    let rel_guid = guid("IFCRELDEFINESBYPROPERTIES", rel_global_id)?;

    let mut properties = vec![single_value(tx, "Station", Value::Real(station))];
    if let Some(value) = incoming_station {
        properties.push(single_value(tx, "IncomingStation", Value::Real(value)));
    }
    if let Some(value) = has_increasing_station {
        properties.push(single_value(tx, "HasIncreasingStation", Value::Bool(value)));
    }

    let mut pset_attrs = vec![Value::Null; 5];
    pset_attrs[0] = pset_guid;
    pset_attrs[2] = Value::Text("Pset_Stationing".into());
    pset_attrs[4] = Value::List(properties.into_iter().map(Value::Ref).collect());
    let pset = tx.create(Entity::new("IFCPROPERTYSET", pset_attrs));

    let mut rel_attrs = vec![Value::Null; 6];
    rel_attrs[0] = rel_guid;
    rel_attrs[4] = Value::List(vec![Value::Ref(referent)]);
    rel_attrs[5] = Value::Ref(pset);
    Ok(tx.create(Entity::new("IFCRELDEFINESBYPROPERTIES", rel_attrs)))
}

/// Stage an `IfcPropertySingleValue` for the stationing set.
fn single_value(tx: &mut Transaction, name: &str, value: Value) -> EntityId {
    let mut attrs = vec![Value::Null; 4];
    attrs[0] = Value::Text(name.into());
    attrs[2] = value;
    tx.create(Entity::new("IFCPROPERTYSINGLEVALUE", attrs))
}
