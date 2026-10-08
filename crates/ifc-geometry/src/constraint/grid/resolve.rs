//! Resolving an `IfcGridPlacement` to a frame in its grid's coordinates
//! (#362).
//!
//! # What IFC4.3 ADD2 states
//!
//! - Location (`IfcVirtualGridIntersection`): "OffsetDistances[1] sets the
//!   offset to IntersectingAxes[1], OffsetDistances[2] sets the offset to
//!   IntersectingAxes[2]", and "the intersection is defined by the offset
//!   curves to the grid axes". "A positive value of distance defines an
//!   offset in the direction which is normal to the curve in the sense of
//!   an anti-clockwise rotation through 90 degrees from the tangent vector
//!   T", that is to the LEFT of the axis, and "This can be reverted by the
//!   SameSense attribute at IfcGridAxis". "OffsetDistances[3] sets the
//!   offset to the virtual intersection in direction of the orientation of
//!   the cross product of IntersectingAxes[1] and the orthogonal complement
//!   of the IntersectingAxes[1]", which for a curve in the grid's XY plane
//!   is `T x (Z x T) = +Z`.
//! - Frame: "The grid axis is positioned within the XY plane of the
//!   position coordinate system defined by the IfcGrid" (`IfcGridAxis`),
//!   and "The IfcGrid local placement ... has to be taken into account for
//!   calculating the absolute placement of the IfcVirtualGridIntersection".
//!   Everything here is in the grid's coordinates; the chain walk composes
//!   the grid's `ObjectPlacement` above it.
//! - Orientation (`IfcGridPlacement`): with no `PlacementRefDirection`, "the
//!   tangent of the first grid axis (PlacementLocation.IntersectingAxes[1])
//!   at the virtual intersection"; with an `IfcDirection`, its
//!   `DirectionRatios`, "only the ratios for x and y are taken into
//!   account"; with a second `IfcVirtualGridIntersection`, "the tangent of
//!   the line between the virtual grid intersection of the PlacementLocation
//!   and the virtual grid intersection of the PlacementRefDirection". The
//!   x-y plane "shall be co-planar to the xy plane of the local placement of
//!   the IfcGrid" and the z-axis "co-linear to the z-axis of the local
//!   placement of the IfcGrid".
//!
//! # Straight and curved axes
//!
//! A straight axis -- an `IfcLine`, an `IfcPolyline` whose points are
//! collinear, an `IfcTrimmedCurve` on a line, an `IfcOffsetCurve2D` of any
//! of these -- is intersected in closed form, as the infinite line it
//! lies on ("virtual" because the axes need not physically cross). Any
//! other axis is curved: it is intersected through the caller's curve
//! evaluator (feature `compile`), or refused by name without one.

use ifc_model::{EntityId, Model, Value};

use super::{grid_slot, GridAxis, GridPlacement, VirtualGridIntersection};
use crate::constraint::placement::LinearResolution;
use crate::constraint::tolerance::model_precision;
use crate::curve::line::Line;
use crate::curve::offset::OffsetCurve2D;
use crate::curve::polyline::Polyline;
use crate::curve::trimmed::TrimmedCurve;
use crate::error::{GeometryError, GeometryResult};
use crate::resource::direction::{resolve_ratios_3d, resolve_unit};
use crate::slots::Slots;
use crate::transform::Transform;
use crate::units::UnitScale;

/// How far apart, as the sine of their angle, two straight axes must be
/// to intersect at a defined point.
const PARALLEL_SINE: f64 = 1e-9;

/// An `IfcGridPlacement` resolved in its grid's coordinates.
#[derive(Debug, Clone, Copy)]
pub(crate) struct GridFrame {
    /// The grid's `ObjectPlacement`.
    pub(crate) grid_placement: EntityId,
    /// The placement's frame in the grid's coordinates, in FILE units.
    pub(crate) local: Transform,
}

/// A virtual grid intersection, in the grid's XY plane, in FILE units.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Intersection {
    /// The grid both axes belong to.
    pub(crate) grid: EntityId,
    /// The intersection of the two offset curves.
    pub(crate) point: [f64; 2],
    /// The unit tangent of the first axis there, in its grid-axis sense.
    pub(crate) tangent: [f64; 2],
    /// `OffsetDistances[3]`, along the grid's Z; zero when absent.
    pub(crate) z: f64,
}

/// One axis of an intersection, as read.
#[derive(Debug, Clone, Copy)]
pub(crate) struct AxisInput {
    /// The `IfcGridAxis`.
    pub(crate) axis: EntityId,
    /// Its `AxisCurve`.
    pub(crate) curve: EntityId,
    /// `IfcGridAxis.SameSense`.
    pub(crate) same_sense: bool,
    /// Its `OffsetDistances` entry, FILE units, to the left of the axis.
    pub(crate) offset: f64,
}

/// A straight axis: the line it lies on, in the curve's own sense.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct StraightAxis {
    /// A point on the line, FILE units.
    pub(crate) point: [f64; 2],
    /// Its unit direction.
    pub(crate) direction: [f64; 2],
}

/// Resolve `placement` in the coordinates of the grid its axes belong to.
///
/// # Errors
///
/// - [`GeometryError::GridAxisWithoutGrid`] for an axis no `IfcGrid` lists;
///   [`GeometryError::GridAxesInDifferentGrids`] for axes from two grids;
/// - [`GeometryError::GridAxesParallel`] for parallel straight axes,
///   [`GeometryError::GridAxesDoNotIntersect`] for curved ones that miss or
///   meet more than once;
/// - [`GeometryError::Unsupported`] naming a curved axis curve when no
///   evaluator was supplied;
/// - [`GeometryError::MissingAttribute`] for a grid without
///   `ObjectPlacement` (IFC4 `IfcGrid` WHERE `HasPlacement`);
/// - [`GeometryError::MissingEntity`] / [`GeometryError::WrongEntityType`]
///   for a dangling or mistyped reference; [`GeometryError::Degenerate`] for
///   a zero reference direction.
pub(crate) fn grid_frame(
    model: &Model,
    units: &UnitScale,
    placement: EntityId,
    linear: LinearResolution<'_>,
) -> GeometryResult<GridFrame> {
    let entity = model.get(placement).ok_or(GeometryError::MissingEntity {
        referrer: placement,
        missing: placement,
    })?;
    let view = GridPlacement::new(placement, entity);
    let location = intersection(model, units, placement, view.location()?, linear)?;

    let x = match view.ref_direction() {
        None => location.tangent,
        Some(reference) => {
            reference_direction(model, units, placement, &location, reference, linear)?
        }
    };
    let origin = [location.point[0], location.point[1], location.z];
    let local = Transform::from_axes(origin, Some([0.0, 0.0, 1.0]), Some([x[0], x[1], 0.0]))
        .ok_or_else(|| GeometryError::Degenerate {
            entity: placement,
            type_name: "IFCGRIDPLACEMENT".into(),
            detail: "the placement's x-axis has no direction in the grid's XY plane".into(),
        })?;

    let grid_entity = model
        .get(location.grid)
        .ok_or(GeometryError::MissingEntity {
            referrer: placement,
            missing: location.grid,
        })?;
    let grid_placement = crate::input::product::Product::new(location.grid, grid_entity)
        .object_placement()
        .ok_or_else(|| GeometryError::MissingAttribute {
            entity: location.grid,
            type_name: "IFCGRID".into(),
            attribute: "ObjectPlacement",
        })?;
    Ok(GridFrame {
        grid_placement,
        local,
    })
}

/// The x direction `PlacementRefDirection` states, in the grid's XY plane.
fn reference_direction(
    model: &Model,
    units: &UnitScale,
    placement: EntityId,
    location: &Intersection,
    reference: EntityId,
    linear: LinearResolution<'_>,
) -> GeometryResult<[f64; 2]> {
    let entity = model.get(reference).ok_or(GeometryError::MissingEntity {
        referrer: placement,
        missing: reference,
    })?;
    let (vector, what) = match entity.type_name.as_ref() {
        "IFCDIRECTION" => {
            let ratios = resolve_ratios_3d(model, placement, reference)?;
            (
                [ratios[0], ratios[1]],
                "an IfcDirection with no x or y ratio",
            )
        }
        "IFCVIRTUALGRIDINTERSECTION" => {
            let other = intersection(model, units, placement, reference, linear)?;
            if other.grid != location.grid {
                return Err(GeometryError::GridAxesInDifferentGrids {
                    intersection: placement,
                    grids: [location.grid, other.grid],
                });
            }
            (
                [
                    other.point[0] - location.point[0],
                    other.point[1] - location.point[1],
                ],
                "a reference intersection that coincides with the placement location",
            )
        }
        other => {
            return Err(GeometryError::WrongEntityType {
                entity: reference,
                actual: other.to_string(),
                expected: "IfcGridPlacementDirectionSelect",
            })
        }
    };
    normalise(vector).ok_or_else(|| GeometryError::Degenerate {
        entity: placement,
        type_name: "IFCGRIDPLACEMENT".into(),
        detail: format!("PlacementRefDirection is {what}"),
    })
}

/// Resolve one `IfcVirtualGridIntersection`.
pub(crate) fn intersection(
    model: &Model,
    units: &UnitScale,
    referrer: EntityId,
    id: EntityId,
    linear: LinearResolution<'_>,
) -> GeometryResult<Intersection> {
    let entity = model.get(id).ok_or(GeometryError::MissingEntity {
        referrer,
        missing: id,
    })?;
    if !entity.is_type("IFCVIRTUALGRIDINTERSECTION") {
        return Err(GeometryError::WrongEntityType {
            entity: id,
            actual: entity.type_name.to_string(),
            expected: "IfcVirtualGridIntersection",
        });
    }
    let view = VirtualGridIntersection::new(id, entity);
    let degenerate = |detail: String| GeometryError::Degenerate {
        entity: id,
        type_name: "IFCVIRTUALGRIDINTERSECTION".into(),
        detail,
    };
    let axes = view.axes()?;
    let [first, second] = axes[..] else {
        return Err(degenerate(format!(
            "IntersectingAxes holds {} axes, not two",
            axes.len()
        )));
    };
    let offsets = view.offsets()?;
    if !(2..=3).contains(&offsets.len()) || offsets.iter().any(|v| !v.is_finite()) {
        return Err(degenerate(format!(
            "OffsetDistances must hold two or three finite lengths, found {offsets:?}"
        )));
    }

    let inputs = [
        axis_input(model, id, first, offsets[0])?,
        axis_input(model, id, second, offsets[1])?,
    ];
    let grids = [owning_grid(model, first)?, owning_grid(model, second)?];
    if grids[0] != grids[1] {
        return Err(GeometryError::GridAxesInDifferentGrids {
            intersection: id,
            grids,
        });
    }

    let precision = model_precision(model)?;
    let straight = [
        straight_axis(model, inputs[0].curve, precision)?,
        straight_axis(model, inputs[1].curve, precision)?,
    ];
    let (point, tangent) = match straight {
        [Some(a), Some(b)] => straight_intersection(id, &inputs, a, b)?,
        _ => curved_intersection(model, units, id, &inputs, straight, linear)?,
    };
    Ok(Intersection {
        grid: grids[0],
        point,
        tangent,
        z: offsets.get(2).copied().unwrap_or(0.0),
    })
}

/// Read one `IntersectingAxes` entry.
fn axis_input(
    model: &Model,
    intersection: EntityId,
    axis: EntityId,
    offset: f64,
) -> GeometryResult<AxisInput> {
    let entity = model.get(axis).ok_or(GeometryError::MissingEntity {
        referrer: intersection,
        missing: axis,
    })?;
    if !entity.is_type("IFCGRIDAXIS") {
        return Err(GeometryError::WrongEntityType {
            entity: axis,
            actual: entity.type_name.to_string(),
            expected: "IfcGridAxis",
        });
    }
    let view = GridAxis::new(axis, entity);
    Ok(AxisInput {
        axis,
        curve: view.curve()?,
        same_sense: view.same_sense(),
        offset,
    })
}

/// The `IfcGrid` that lists `axis` in `UAxes`, `VAxes` or `WAxes`.
fn owning_grid(model: &Model, axis: EntityId) -> GeometryResult<EntityId> {
    let lists = |entity: &ifc_model::Entity, slot: usize| {
        matches!(entity.attributes.get(slot), Some(Value::List(items))
            if items.iter().any(|item| item.as_ref_id() == Some(axis)))
    };
    let mut owners = model
        .of_type("IFCGRID")
        .filter(|(_, entity)| {
            [grid_slot::U_AXES, grid_slot::V_AXES, grid_slot::W_AXES]
                .into_iter()
                .any(|slot| lists(entity, slot))
        })
        .map(|(id, _)| id);
    let Some(grid) = owners.next() else {
        return Err(GeometryError::GridAxisWithoutGrid { axis });
    };
    if let Some(other) = owners.next() {
        // IfcGridAxis WR2: one list of one grid.
        return Err(GeometryError::GridAxesInDifferentGrids {
            intersection: axis,
            grids: [grid, other],
        });
    }
    Ok(grid)
}

/// The line a straight axis curve lies on, in the curve's own sense, or
/// `None` for a curved one.
pub(crate) fn straight_axis(
    model: &Model,
    curve: EntityId,
    precision: f64,
) -> GeometryResult<Option<StraightAxis>> {
    let entity = model.get(curve).ok_or(GeometryError::MissingEntity {
        referrer: curve,
        missing: curve,
    })?;
    let degenerate = |detail: &str| GeometryError::Degenerate {
        entity: curve,
        type_name: entity.type_name.to_string(),
        detail: detail.to_owned(),
    };
    match entity.type_name.as_ref() {
        "IFCLINE" => {
            let view = Line::new(curve, entity);
            let point = xy(&view.point(model)?.coordinates()?);
            let vector_ref = view.direction_vector_ref()?;
            let vector = model.get(vector_ref).ok_or(GeometryError::MissingEntity {
                referrer: curve,
                missing: vector_ref,
            })?;
            let orientation = Slots::new(vector_ref, vector).req_ref(0, "Orientation")?;
            let ratios = resolve_unit(model, vector_ref, orientation)?;
            let direction = normalise([ratios[0], ratios[1]])
                .ok_or_else(|| degenerate("the line's direction has no x or y component"))?;
            Ok(Some(StraightAxis { point, direction }))
        }
        "IFCPOLYLINE" => {
            let points: Vec<[f64; 2]> = Polyline::new(curve, entity)
                .points(model)?
                .iter()
                .map(|point| point.coordinates().map(|c| xy(&c)))
                .collect::<GeometryResult<_>>()?;
            let start = points[0];
            let end = points[points.len() - 1];
            let direction = normalise([end[0] - start[0], end[1] - start[1]])
                .ok_or_else(|| degenerate("the polyline starts and ends at the same point"))?;
            // Collinear within the model's Precision: straight. Otherwise a
            // bent polyline, which is curved for our purposes.
            let off_line = |p: &[f64; 2]| {
                ((p[0] - start[0]) * direction[1] - (p[1] - start[1]) * direction[0]).abs()
            };
            if points.iter().all(|p| off_line(p) <= precision) {
                Ok(Some(StraightAxis {
                    point: start,
                    direction,
                }))
            } else {
                Ok(None)
            }
        }
        "IFCTRIMMEDCURVE" => {
            let view = TrimmedCurve::new(curve, entity);
            let Some(basis) = straight_axis(model, view.basis_curve_ref()?, precision)? else {
                return Ok(None);
            };
            // SenseAgreement FALSE runs the trimmed curve against its basis.
            let sense = if view.sense_agreement()? { 1.0 } else { -1.0 };
            Ok(Some(StraightAxis {
                point: basis.point,
                direction: basis.direction.map(|v| v * sense),
            }))
        }
        "IFCOFFSETCURVE2D" => {
            let view = OffsetCurve2D::new(curve, entity);
            let Some(basis) = straight_axis(model, view.basis_curve_ref()?, precision)? else {
                return Ok(None);
            };
            // ISO 10303-42 offset_curve_2d: a positive distance lies along
            // the orthogonal complement of the tangent, to the left.
            let distance = view.distance()?;
            let left = left_of(basis.direction);
            Ok(Some(StraightAxis {
                point: [
                    basis.point[0] + distance * left[0],
                    basis.point[1] + distance * left[1],
                ],
                direction: basis.direction,
            }))
        }
        _ => Ok(None),
    }
}

/// Intersect two straight axes, each offset to its left.
fn straight_intersection(
    id: EntityId,
    inputs: &[AxisInput; 2],
    a: StraightAxis,
    b: StraightAxis,
) -> GeometryResult<([f64; 2], [f64; 2])> {
    let [(qa, da), (qb, db)] = [(a, &inputs[0]), (b, &inputs[1])].map(|(line, input)| {
        let sense = if input.same_sense { 1.0 } else { -1.0 };
        let direction = line.direction.map(|v| v * sense);
        let left = left_of(direction);
        let point = [
            line.point[0] + input.offset * left[0],
            line.point[1] + input.offset * left[1],
        ];
        (point, direction)
    });
    let cross = da[0] * db[1] - da[1] * db[0];
    if !cross.is_finite() || cross.abs() <= PARALLEL_SINE {
        return Err(GeometryError::GridAxesParallel {
            intersection: id,
            axes: [inputs[0].axis, inputs[1].axis],
        });
    }
    let w = [qb[0] - qa[0], qb[1] - qa[1]];
    let s = (w[0] * db[1] - w[1] * db[0]) / cross;
    Ok(([qa[0] + s * da[0], qa[1] + s * da[1]], da))
}

/// Intersect axes of which at least one is curved.
#[cfg_attr(not(feature = "compile"), allow(unused_variables))]
fn curved_intersection(
    model: &Model,
    units: &UnitScale,
    id: EntityId,
    inputs: &[AxisInput; 2],
    straight: [Option<StraightAxis>; 2],
    linear: LinearResolution<'_>,
) -> GeometryResult<([f64; 2], [f64; 2])> {
    #[cfg(feature = "compile")]
    if let Some(derivation) = linear.derivation {
        return super::curved::intersect(model, units, id, inputs, straight, derivation.evaluator);
    }
    let curved = if straight[0].is_none() {
        inputs[0]
    } else {
        inputs[1]
    };
    let type_name = model
        .get(curved.curve)
        .map_or_else(String::new, |entity| entity.type_name.to_string());
    Err(GeometryError::Unsupported {
        entity: curved.curve,
        type_name,
        detail: "a curved IfcGridAxis is intersected through a CurveEvaluator, and none was \
                 supplied (feature `compile`: LoweringSession::with_curve_evaluator or \
                 product_world_transform_with_evaluator)",
    })
}

/// The first two coordinates of a point: the grid's XY plane.
fn xy(coordinates: &[f64]) -> [f64; 2] {
    [
        coordinates.first().copied().unwrap_or(0.0),
        coordinates.get(1).copied().unwrap_or(0.0),
    ]
}

/// The left normal: `v` rotated anti-clockwise through 90 degrees.
pub(crate) fn left_of(v: [f64; 2]) -> [f64; 2] {
    [-v[1], v[0]]
}

/// `v` scaled to unit length, or `None` when it has none.
pub(crate) fn normalise(v: [f64; 2]) -> Option<[f64; 2]> {
    let length = (v[0] * v[0] + v[1] * v[1]).sqrt();
    (length.is_finite() && length > 1e-12).then(|| [v[0] / length, v[1] / length])
}
