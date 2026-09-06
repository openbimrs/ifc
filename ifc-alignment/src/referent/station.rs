//! Stationing referents and station equations.
//!
//! Stationing is a labeling system layered onto distance-along, not distance
//! itself: a `Pset_Stationing.Station` value can jump, run backwards, or
//! restart at an `IncomingStation`/`Station` pair (a "station equation" --
//! e.g. two survey crews meeting mid-alignment with a small chainage gap or
//! overlap). Treating station as interchangeable with distance-along is
//! exactly the linear-referencing bug this module exists to prevent: this
//! type never returns a bare number, only the mapping.

use ifc_model::value::Value;
use ifc_model::{EntityId, Model};

use crate::error::{AlignmentError, AlignmentResult};
use crate::horizontal::AlignmentUnits;

/// One `IfcReferent` carrying `Pset_Stationing`, resolved to its distance
/// along the alignment and station value.
///
/// `incoming_station` is `Some` exactly when the referent marks a station
/// equation: the station value that would have continued from the *previous*
/// segment, immediately before it is replaced by `station` at this point.
#[derive(Debug, Clone, PartialEq)]
pub struct StationEquation {
    pub referent: EntityId,
    pub distance_along: f64,
    pub incoming_station: Option<f64>,
    pub station: f64,
    pub has_increasing_station: bool,
}

/// Resolve every `IfcReferent` in `model` that carries a `Pset_Stationing`
/// property set into its distance-along/station mapping.
///
/// Distance along comes from the referent's own `IfcLinearPlacement`
/// (`DistanceAlong` of the underlying `IfcPointByDistanceExpression`), not
/// from station: the two are related only by this returned table, never
/// interchangeable.
pub fn station_equations(
    model: &Model,
    units: AlignmentUnits,
) -> AlignmentResult<Vec<StationEquation>> {
    let mut out = Vec::new();
    for (id, entity) in model.iter() {
        if !entity.is_type("IFCREFERENT") {
            continue;
        }
        if let Some(equation) = resolve_referent_stationing(model, id, units)? {
            out.push(equation);
        }
    }
    // Stable, order-independent output: callers reason about a mapping, and
    // model iteration order is an implementation detail of storage, not of
    // the alignment's stationing scheme.
    out.sort_by(|a, b| {
        a.distance_along
            .partial_cmp(&b.distance_along)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    Ok(out)
}

fn resolve_referent_stationing(
    model: &Model,
    referent: EntityId,
    units: AlignmentUnits,
) -> AlignmentResult<Option<StationEquation>> {
    let Some(pset) = find_pset_stationing(model, referent)? else {
        return Ok(None);
    };
    let (station, incoming_station, has_increasing_station) =
        read_stationing_properties(model, pset, referent)?;
    let distance_along = referent_distance_along(model, referent, units)?;

    Ok(Some(StationEquation {
        referent,
        distance_along,
        incoming_station: incoming_station.map(|v| v * units.length_to_metres),
        station: station * units.length_to_metres,
        has_increasing_station,
    }))
}

/// Find the `Pset_Stationing` `IfcPropertySet` attached to `referent` via
/// `IfcRelDefinesByProperties`, if any.
fn find_pset_stationing(model: &Model, referent: EntityId) -> AlignmentResult<Option<EntityId>> {
    for (_id, entity) in model.iter() {
        if !entity.is_type("IFCRELDEFINESBYPROPERTIES") {
            continue;
        }
        // IfcRelDefinesByProperties: inherited GlobalId/OwnerHistory/Name/
        // Description are slots 0..3; RelatedObjects and
        // RelatingPropertyDefinition are this declaration's own slots 4..5.
        let values = &entity.attributes;
        let related = values.get(4).and_then(Value::as_list).unwrap_or(&[]);
        let targets_referent = related.iter().any(|v| v.as_ref_id() == Some(referent));
        if !targets_referent {
            continue;
        }
        let Some(pset_id) = values.get(5).and_then(Value::as_ref_id) else {
            continue;
        };
        let Some(pset_entity) = model.get(pset_id) else {
            continue;
        };
        if !pset_entity.is_type("IFCPROPERTYSET") {
            continue;
        }
        // IfcPropertySet: inherited slots 0..3, own Name is slot 2 (already
        // covered by IfcRoot); HasProperties is this declaration's own slot 4.
        let is_stationing = pset_entity
            .attributes
            .get(2)
            .and_then(Value::as_text)
            .is_some_and(|name| name == "Pset_Stationing");
        if is_stationing {
            return Ok(Some(pset_id));
        }
    }
    Ok(None)
}

fn read_stationing_properties(
    model: &Model,
    pset: EntityId,
    referent: EntityId,
) -> AlignmentResult<(f64, Option<f64>, bool)> {
    let pset_entity = model
        .get(pset)
        .ok_or(AlignmentError::MissingEntity { entity: pset })?;
    let properties = pset_entity
        .attributes
        .get(4)
        .and_then(Value::as_list)
        .unwrap_or(&[]);

    let mut station = None;
    let mut incoming_station = None;
    let mut has_increasing_station = true; // spec default: absent means true.

    for property in properties {
        let Some(property_id) = property.as_ref_id() else {
            continue;
        };
        let Some(property_entity) = model.get(property_id) else {
            continue;
        };
        if !property_entity.is_type("IFCPROPERTYSINGLEVALUE") {
            continue;
        }
        // IfcPropertySingleValue: IfcProperty own Name/Specification are
        // slots 0..1 (IfcPropertyAbstraction contributes none); NominalValue
        // and Unit are this declaration's own slots 2..3.
        let name = property_entity
            .attributes
            .first()
            .and_then(Value::as_text)
            .ok_or(AlignmentError::InvalidAttribute {
                entity: property_id,
                index: 0,
                name: "Name",
            })?;
        let nominal = property_entity.attributes.get(2);
        match name {
            "Station" => {
                station = nominal.and_then(|v| v.unwrap_typed().as_f64());
            }
            "IncomingStation" => {
                incoming_station = nominal.and_then(|v| v.unwrap_typed().as_f64());
            }
            "HasIncreasingStation" => {
                if let Some(value) = nominal.and_then(|v| v.unwrap_typed().as_bool()) {
                    has_increasing_station = value;
                }
            }
            _ => {}
        }
    }

    let station = station.ok_or(AlignmentError::SemanticViolation {
        entity: Some(referent),
        rule: "Pset_Stationing.Station is required to interpret a station equation",
    })?;
    Ok((station, incoming_station, has_increasing_station))
}

/// `IfcReferent.ObjectPlacement`, resolved to an absolute distance along its
/// basis curve. A station equation anchored to a *parameter* rather than a
/// length is refused: parameters are basis-curve-relative and not comparable
/// across segments the way an absolute distance-along is.
fn referent_distance_along(
    model: &Model,
    referent: EntityId,
    units: AlignmentUnits,
) -> AlignmentResult<f64> {
    let entity = model
        .get(referent)
        .ok_or(AlignmentError::MissingEntity { entity: referent })?;
    let placement_id = entity.attributes.get(5).and_then(Value::as_ref_id).ok_or(
        AlignmentError::InvalidAttribute {
            entity: referent,
            index: 5,
            name: "ObjectPlacement",
        },
    )?;
    let placement = crate::placement::resolve_linear_placement(model, placement_id, units)?;
    match placement.relative_placement.distance_along {
        crate::placement::CurveMeasure::Length(value) => Ok(value),
        crate::placement::CurveMeasure::Parameter(_) => Err(AlignmentError::Unsupported {
            entity: referent,
            type_name: "IFCPOINTBYDISTANCEEXPRESSION".to_owned(),
            detail: "a station equation requires an absolute DistanceAlong, not a curve parameter",
        }),
    }
}
