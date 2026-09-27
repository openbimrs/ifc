//! Straight-segment basis curves, lowered to the neutral polyline.
//!
//! A polyline's arc length is a finite sum of segment lengths, so an
//! evaluator can turn a distance along it into a parameter exactly. That
//! makes `IfcPolyline` and a line-only `IfcIndexedPolyCurve` safe to derive
//! a placement from, unlike an ellipse or a B-spline whose arc length needs
//! an integral.
//!
//! # Parameterisation
//!
//! IFC4 ADD2 TC1 (`IfcPolyline`, after ISO 10303-42) parameterises the
//! `i`-th of `n - 1` segments over `i - 1 <= u <= i`, a total range of
//! `[0, n - 1]`. The neutral polyline uses the same convention (the integer
//! part selects the segment), so a native `IfcCurveMeasureSelect` parameter
//! crosses unchanged. The closing duplicate of a closed polyline is kept as
//! an ordinary last vertex for the same reason: dropping it would shorten
//! the parameter range by one segment.
//!
//! For `IfcIndexedPolyCurve` the same document states a parametric length of
//! 1.0 per straight segment when `Segments` is absent. It does not say how a
//! multi-point `IfcLineIndex` divides its parametric length, so a parameter
//! on such a curve is refused rather than guessed; a distance is unaffected.
//!
//! # Zero-length segments
//!
//! A repeated point has no direction, so distance along it is ambiguous.
//! It is refused here, naming the curve and the segment, instead of being
//! left to surface as an anonymous kernel error.

use axiolid_core::Point3;
use axiolid_curve::{Curve3, Polyline3};
use ifc_model::{Entity, EntityId, Model};

use crate::curve::polyline::{IndexedPolyCurve, PolySegment, Polyline};
use crate::error::{GeometryError, GeometryResult};
use crate::resource::point::CartesianPointList;
use crate::units::UnitScale;

/// Lower an `IfcPolyline` basis curve, in metres.
pub(super) fn polyline(
    model: &Model,
    units: &UnitScale,
    basis: EntityId,
    entity: &Entity,
) -> GeometryResult<Curve3> {
    let view = Polyline::new(basis, entity);
    let raw = view
        .points(model)?
        .iter()
        .map(|point| point.coordinates_3d())
        .collect::<GeometryResult<Vec<_>>>()?;
    neutral(units, basis, entity, raw)
}

/// Lower a line-only `IfcIndexedPolyCurve` basis curve, in metres.
///
/// `parameter_requested` says the caller will evaluate by native parameter,
/// which is only defined where IFC states one per straight segment.
pub(super) fn indexed_polycurve(
    model: &Model,
    units: &UnitScale,
    basis: EntityId,
    entity: &Entity,
    parameter_requested: bool,
) -> GeometryResult<Curve3> {
    let view = IndexedPolyCurve::new(basis, entity);
    let list: Vec<[f64; 3]> = match view.points(model)? {
        CartesianPointList::TwoD(list) => list
            .coordinates()?
            .into_iter()
            .map(|[x, y]| [x, y, 0.0])
            .collect(),
        CartesianPointList::ThreeD(list) => list.coordinates()?,
    };
    if !view.has_explicit_segments() {
        return neutral(units, basis, entity, list);
    }

    let mut indices: Vec<usize> = Vec::new();
    for segment in view.segments(list.len())? {
        let PolySegment::Line(run) = segment else {
            return Err(GeometryError::Unsupported {
                entity: basis,
                type_name: entity.type_name.to_string(),
                detail: "an IfcArcIndex segment has no exact distance along a polyline; \
                         only line-only indexed poly-curves derive a placement",
            });
        };
        if parameter_requested && run.len() > 2 {
            return Err(GeometryError::Unsupported {
                entity: basis,
                type_name: entity.type_name.to_string(),
                detail: "a multi-point IfcLineIndex has no stated parametric length, \
                         so a parameter along it is ambiguous; state a distance",
            });
        }
        match indices.last() {
            None => indices.extend_from_slice(&run),
            // `IfcConsecutiveSegments`: each segment starts where the last ended.
            Some(end) if Some(end) == run.first() => indices.extend_from_slice(&run[1..]),
            Some(_) => {
                return Err(GeometryError::Degenerate {
                    entity: basis,
                    type_name: entity.type_name.to_string(),
                    detail: "Segments are not consecutive (WHERE rule Consecutive), \
                             so the curve has no single path to measure along"
                        .to_owned(),
                });
            }
        }
    }
    let raw = indices.into_iter().map(|i| list[i]).collect();
    neutral(units, basis, entity, raw)
}

/// Scale to metres and refuse a zero-length segment by name.
fn neutral(
    units: &UnitScale,
    basis: EntityId,
    entity: &Entity,
    raw: Vec<[f64; 3]>,
) -> GeometryResult<Curve3> {
    if raw.len() < 2 {
        return Err(GeometryError::Degenerate {
            entity: basis,
            type_name: entity.type_name.to_string(),
            detail: format!(
                "a path of {} point(s) has no segment to measure along",
                raw.len()
            ),
        });
    }
    // 1-based, as a reader of the file counts segments.
    if let Some(segment) = raw.windows(2).position(|pair| pair[0] == pair[1]) {
        return Err(GeometryError::Degenerate {
            entity: basis,
            type_name: entity.type_name.to_string(),
            detail: format!(
                "segment {} has zero length, so distance along the curve is ambiguous",
                segment + 1
            ),
        });
    }
    let points = raw
        .into_iter()
        .map(|[x, y, z]| Point3::from_array([units.length(x), units.length(y), units.length(z)]))
        .collect();
    Ok(Curve3::Polyline(Polyline3 {
        points,
        closed: false,
    }))
}
