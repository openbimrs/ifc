//! Where a material layer set usage measures its layers from (#455).
//!
//! `IfcMaterialLayerSetUsage` says how thick each layer is and how far the
//! stack is offset, but not from what. The reference comes from the product
//! (IFC4 ADD2 TC1, the "Material Layer Set Usage" clauses of `IfcWall`,
//! `IfcSlab` and `IfcPlate`):
//!
//! - **AXIS2, walls.** Layers are measured from the wall's `Axis`
//!   representation, a curve in the object coordinate system. The offset
//!   moves the base line along the local y axis, positive towards +y, and a
//!   `POSITIVE` sense then builds the layers toward +y. The extrusion may run
//!   across the wall (perpendicular to y) or through its thickness (parallel
//!   to y, an edited profile); an oblique one is refused. `IfcWallStandardCase`
//!   additionally keeps the axis parallel to the local x axis, which this
//!   module checks for every wall rather than assuming.
//! - **AXIS3, slabs and plates.** Layers are measured from the plane of the
//!   body extrusion's profile, along that plane's normal, which is the z axis
//!   of the `IfcExtrudedAreaSolid.Position` frame. It does not depend on
//!   `ExtrudedDirection`, so a roof slab extruded along a leaning direction
//!   still has its layers parallel to the profile plane.
//! - **AXIS1** has no use definition for any element in that release.
//!
//! The usage attributes `ForLayerSet`, `LayerSetDirection`, `DirectionSense`
//! and `OffsetFromReferenceLine` exist in every release; IFC4 and IFC4X3 add
//! `ReferenceExtent`, which only matters for fitting a body, not here.
//! `LayerThickness` is positive in IFC2X3 and non-negative from IFC4, so a
//! zero-thick layer is legal and yields two equal boundaries.
//!
//! The answer is a set of exact boundary planes, not split bodies: this crate
//! never tessellates or executes booleans. Everything is in the product's
//! object frame, in metres. The `Position` is optional from IFC4 (absent
//! means identity) and a clipping chain (`IfcBooleanClippingResult`) is
//! walked to its extrusion, because clipping trims the body without changing
//! which plane the layers hang from. A body of several items (an interrupted
//! wall) is accepted when they share the `Position` orientation, the profile plane and the
//! extrusion direction.

use ifc_model::{EntityId, Model};

use super::material_usage::{DirectionSense, LayerSetDirection};
use super::representation::{select_product_representation, Representation, RepresentationPurpose};
use crate::curve::polyline::Polyline;
use crate::error::{GeometryError, GeometryResult};
use crate::resource::placement::axis_placement_transform;
use crate::resource::resolve;
use crate::solid::boolean::BooleanClippingResult;
use crate::solid::swept::area::ExtrudedAreaSolid;
use crate::transform::Transform;
use crate::units::UnitScale;

/// Longest clipping chain walked to the extrusion.
const MAX_CLIPPING_DEPTH: usize = 64;
/// Wall-axis straightness and extent tolerance, metres.
const AXIS_TOLERANCE: f64 = 1e-6;
/// Tolerance on unit direction components.
const DIRECTION_TOLERANCE: f64 = 1e-9;
const AXIS2_CLASSES: [&str; 2] = ["IFCWALL", "IFCWALLSTANDARDCASE"];
const AXIS3_CLASSES: [&str; 4] = [
    "IFCSLAB",
    "IFCSLABSTANDARDCASE",
    "IFCPLATE",
    "IFCPLATESTANDARDCASE",
];

/// Where a material layer set usage measures its layers from, in the
/// product's object coordinate system.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct LayerSetReference {
    /// The product the reference was read from.
    pub product: EntityId,
    /// The usage's layer direction.
    pub direction: LayerSetDirection,
    /// Unit normal of every layer boundary plane, object frame: +y for AXIS2,
    /// the extrusion `Position`'s z for AXIS3.
    pub normal: [f64; 3],
    /// Signed distance, metres, along `normal` from the product origin to the
    /// reference line (AXIS2) or plane (AXIS3).
    pub distance: f64,
    /// The Axis `IfcPolyline` (AXIS2) or the `IfcExtrudedAreaSolid` (AXIS3)
    /// the reference was read from.
    pub source: EntityId,
    product_type: String,
}

/// The boundary planes of a layer stack along a [`LayerSetReference`] normal.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct LayerBoundaries {
    /// Unit normal of the planes, object frame.
    pub normal: [f64; 3],
    /// n + 1 signed distances along `normal`, metres, non-decreasing (equal
    /// where a layer is zero-thick).
    pub boundaries: Vec<f64>,
    /// `order[k]` is the index in `IfcMaterialLayerSet.MaterialLayers` of the
    /// layer between `boundaries[k]` and `boundaries[k + 1]`.
    pub order: Vec<usize>,
}

fn unsupported(id: EntityId, type_name: &str, detail: &'static str) -> GeometryError {
    GeometryError::Unsupported {
        entity: id,
        type_name: type_name.to_string(),
        detail,
    }
}

fn degenerate(id: EntityId, type_name: &str, detail: impl Into<String>) -> GeometryError {
    GeometryError::Degenerate {
        entity: id,
        type_name: type_name.to_string(),
        detail: detail.into(),
    }
}

/// Read where the layers of `product`'s usage are measured from.
///
/// Only `IfcWall`/`IfcWallStandardCase` along AXIS2 and
/// `IfcSlab`/`IfcPlate` (and their standard cases) along AXIS3 are
/// interpreted; everything else is [`GeometryError::Unsupported`]. The body
/// is one or several coaxial `IfcExtrudedAreaSolid` items (an interrupted
/// wall), each optionally clipped; an item that disagrees is refused by name.
pub fn layer_set_reference(
    model: &Model,
    units: &UnitScale,
    product: EntityId,
    direction: LayerSetDirection,
) -> GeometryResult<LayerSetReference> {
    let entity = model.get(product).ok_or(GeometryError::MissingEntity {
        referrer: product,
        missing: product,
    })?;
    let product_type = entity.type_name.to_string();
    let classes: &[&str] =
        match direction {
            LayerSetDirection::Axis2 => &AXIS2_CLASSES,
            LayerSetDirection::Axis3 => &AXIS3_CLASSES,
            _ => return Err(unsupported(
                product,
                &product_type,
                "LayerSetDirection AXIS1 has no use definition for any element in IFC4 ADD2 TC1; \
                 only AXIS2 walls and AXIS3 slabs and plates are interpreted",
            )),
        };
    if !classes.iter().any(|class| entity.is_type(class)) {
        return Err(unsupported(
            product,
            &product_type,
            "a material layer set usage is interpreted along AXIS2 for IfcWall and \
             IfcWallStandardCase, and along AXIS3 for IfcSlab, IfcSlabStandardCase, IfcPlate \
             and IfcPlateStandardCase, only",
        ));
    }

    let body = select_product_representation(model, product, RepresentationPurpose::Body)?
        .ok_or_else(|| GeometryError::MissingAttribute {
            entity: product,
            type_name: product_type.clone(),
            attribute: "Body representation",
        })?;
    let (extrusion_id, local, sweep) = coaxial_frame(model, units, body, direction)?;

    let (normal, distance, source) = match direction {
        LayerSetDirection::Axis2 => {
            let along_y = sweep[1].abs();
            if along_y > DIRECTION_TOLERANCE && along_y < 1.0 - DIRECTION_TOLERANCE {
                return Err(unsupported(
                    extrusion_id,
                    "IFCEXTRUDEDAREASOLID",
                    "the extrusion is neither perpendicular nor parallel to the AXIS2 layer \
                     direction (the product's local y axis), so the layers are not slabs \
                     along it",
                ));
            }
            let (distance, polyline) = wall_axis(model, units, product, &product_type)?;
            ([0.0, 1.0, 0.0], distance, polyline)
        }
        _ => {
            let normal = local.basis[2];
            if dot(sweep, normal).abs() <= DIRECTION_TOLERANCE {
                return Err(degenerate(
                    extrusion_id,
                    "IFCEXTRUDEDAREASOLID",
                    "ExtrudedDirection lies in the profile plane",
                ));
            }
            (normal, dot(normal, local.origin), extrusion_id)
        }
    };
    Ok(LayerSetReference {
        product,
        direction,
        normal,
        distance,
        source,
        product_type,
    })
}

/// The id of the first item, and the frame and unit sweep direction shared by
/// every item of `body`.
///
/// Each item is one `IfcExtrudedAreaSolid`, through any clipping chain. An
/// interrupted wall is several of them, each positioned along the wall; they
/// must share the `Position` orientation and profile plane (origins may slide
/// within it) and the extrusion direction, or the layers would not hang from
/// one plane. The depth is not compared: the reference reads neither it nor
/// the profile. Along AXIS2 the origins may not differ along the layer
/// direction (the object y axis) either: pieces slide along or up the wall,
/// not across its layers. A refusal names the body item.
fn coaxial_frame(
    model: &Model,
    units: &UnitScale,
    body: EntityId,
    direction: LayerSetDirection,
) -> GeometryResult<(EntityId, Transform, [f64; 3])> {
    let representation = model.get(body).ok_or(GeometryError::MissingEntity {
        referrer: body,
        missing: body,
    })?;
    let items = Representation::new(body, representation).items()?;
    if items.is_empty() {
        return Err(unsupported(
            body,
            &representation.type_name,
            "a layered body must hold at least one item",
        ));
    }
    let mut first: Option<(EntityId, Transform, [f64; 3])> = None;
    for item in items {
        let (id, extrusion) = extrusion_of(model, body, item)?;
        let local = match extrusion.base().position() {
            Some(position) => {
                let placement = model.get(position).ok_or(GeometryError::MissingEntity {
                    referrer: id,
                    missing: position,
                })?;
                axis_placement_transform(model, position, placement)?.to_metres(units)
            }
            None => Transform::identity(),
        };
        let sweep = resolve::direction(model, id, extrusion.extruded_direction()?)?.unit()?;
        let sweep = normalised(local.apply_direction(sweep));
        match &first {
            None => first = Some((id, local, sweep)),
            Some((_, frame, first_sweep)) => {
                let shift = [0, 1, 2].map(|i| local.origin[i] - frame.origin[i]);
                let across = matches!(direction, LayerSetDirection::Axis2)
                    && shift[1].abs() > AXIS_TOLERANCE;
                let same_frame = !across
                    && dot(frame.basis[2], shift).abs() <= AXIS_TOLERANCE
                    && frame
                        .basis
                        .iter()
                        .zip(&local.basis)
                        .all(|(a, b)| vectors_agree(*a, *b));
                if !same_frame || !vectors_agree(*first_sweep, sweep) {
                    return Err(unsupported(
                        item,
                        "body item",
                        "the body items are not coaxial: this item differs in Position \
                         orientation, profile plane, offset across the layers or \
                         ExtrudedDirection from the first item, so the layers have no single \
                         reference plane",
                    ));
                }
            }
        }
    }
    let (id, local, sweep) = first.expect("a body with items has a first");
    Ok((id, local, sweep))
}

fn vectors_agree(a: [f64; 3], b: [f64; 3]) -> bool {
    a.iter()
        .zip(&b)
        .all(|(x, y)| (x - y).abs() <= AXIS_TOLERANCE)
}

/// The extrusion of one body item, through any clipping chain.
fn extrusion_of(
    model: &Model,
    body: EntityId,
    first: EntityId,
) -> GeometryResult<(EntityId, ExtrudedAreaSolid<'_>)> {
    let mut id = first;
    for _ in 0..=MAX_CLIPPING_DEPTH {
        let item = model.get(id).ok_or(GeometryError::MissingEntity {
            referrer: body,
            missing: id,
        })?;
        if item.is_type("IFCBOOLEANCLIPPINGRESULT") {
            let clipping = BooleanClippingResult::new(id, item);
            clipping.checked_operator()?;
            id = clipping.base().first_operand()?;
        } else if item.is_type("IFCEXTRUDEDAREASOLID") {
            return Ok((id, ExtrudedAreaSolid::new(id, item)));
        } else {
            return Err(unsupported(
                id,
                &item.type_name,
                "a layered body must be IfcExtrudedAreaSolid items, optionally clipped by \
                 IfcBooleanClippingResult",
            ));
        }
    }
    Err(GeometryError::ChainTooDeep {
        entity: first,
        kind: "boolean clipping",
        limit: MAX_CLIPPING_DEPTH,
    })
}

/// The y of the wall's straight Axis polyline, metres, and the polyline.
fn wall_axis(
    model: &Model,
    units: &UnitScale,
    product: EntityId,
    product_type: &str,
) -> GeometryResult<(f64, EntityId)> {
    let axis = select_product_representation(model, product, RepresentationPurpose::Axis)?
        .ok_or_else(|| GeometryError::MissingAttribute {
            entity: product,
            type_name: product_type.to_string(),
            attribute: "Axis representation",
        })?;
    let representation = model.get(axis).ok_or(GeometryError::MissingEntity {
        referrer: product,
        missing: axis,
    })?;
    let items = Representation::new(axis, representation).items()?;
    let [curve] = items.as_slice() else {
        return Err(unsupported(
            axis,
            &representation.type_name,
            "the Axis representation must hold exactly one curve",
        ));
    };
    let entity = model.get(*curve).ok_or(GeometryError::MissingEntity {
        referrer: axis,
        missing: *curve,
    })?;
    if !entity.is_type("IFCPOLYLINE") {
        return Err(unsupported(
            *curve,
            &entity.type_name,
            "only an IfcPolyline wall axis is interpreted; trimmed, indexed and curved axes \
             are not yet",
        ));
    }
    let mut points = Vec::new();
    for point in Polyline::new(*curve, entity).points(model)? {
        let coordinates = point.coordinates()?;
        let [x, y, ..] = coordinates[..] else {
            return Err(degenerate(
                point.id(),
                "IFCCARTESIANPOINT",
                "an axis point has no y coordinate",
            ));
        };
        points.push([units.length(x), units.length(y)]);
    }
    let y0 = points[0][1];
    if points.iter().any(|p| (p[1] - y0).abs() > AXIS_TOLERANCE) {
        return Err(unsupported(
            *curve,
            "IFCPOLYLINE",
            "the wall axis is not parallel to the product's local x axis",
        ));
    }
    let (low, high) = points
        .iter()
        .fold((f64::MAX, f64::MIN), |(l, h), p| (l.min(p[0]), h.max(p[0])));
    if high - low <= AXIS_TOLERANCE {
        return Err(degenerate(
            *curve,
            "IFCPOLYLINE",
            "the wall axis has no extent along the product's local x axis",
        ));
    }
    Ok((y0, *curve))
}

impl LayerSetReference {
    /// The boundary planes of a stack of layers.
    ///
    /// `offset` is `OffsetFromReferenceLine` and `thicknesses` the
    /// `LayerThickness` values in set order, both in project length units.
    /// `POSITIVE` builds from the offset reference toward +`normal`;
    /// `NEGATIVE` toward -`normal`, and the result is reported ascending, so
    /// `order` lists the layers last to first.
    pub fn boundaries(
        &self,
        units: &UnitScale,
        sense: DirectionSense,
        offset: f64,
        thicknesses: &[f64],
    ) -> GeometryResult<LayerBoundaries> {
        let fail = |detail: String| degenerate(self.product, &self.product_type, detail);
        if thicknesses.is_empty() {
            return Err(fail("a material layer set holds no layer".into()));
        }
        if !offset.is_finite() {
            return Err(fail("OffsetFromReferenceLine is not finite".into()));
        }
        let sign = match sense {
            DirectionSense::Positive => 1.0,
            DirectionSense::Negative => -1.0,
        };
        let mut at = self.distance + units.length(offset);
        let mut boundaries = vec![at];
        for (i, &thickness) in thicknesses.iter().enumerate() {
            if !thickness.is_finite() || thickness < 0.0 {
                return Err(fail(format!(
                    "layer thickness {i} is negative or not finite"
                )));
            }
            at += sign * units.length(thickness);
            boundaries.push(at);
        }
        let mut order: Vec<usize> = (0..thicknesses.len()).collect();
        if sign < 0.0 {
            boundaries.reverse();
            order.reverse();
        }
        Ok(LayerBoundaries {
            normal: self.normal,
            boundaries,
            order,
        })
    }
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn normalised(v: [f64; 3]) -> [f64; 3] {
    let length = dot(v, v).sqrt();
    v.map(|c| c / length)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reference(distance: f64) -> LayerSetReference {
        LayerSetReference {
            product: EntityId(30),
            direction: LayerSetDirection::Axis2,
            normal: [0.0, 1.0, 0.0],
            distance,
            source: EntityId(40),
            product_type: "IFCWALL".into(),
        }
    }

    fn mm() -> UnitScale {
        UnitScale {
            length_to_metres: 0.001,
            ..UnitScale::SI
        }
    }

    fn close(actual: &[f64], expected: &[f64]) {
        assert_eq!(actual.len(), expected.len(), "{actual:?}");
        for (a, e) in actual.iter().zip(expected) {
            assert!((a - e).abs() < 1e-12, "{actual:?} != {expected:?}");
        }
    }

    const CAVITY: [f64; 4] = [115.0, 100.0, 40.0, 115.0];

    #[test]
    fn a_positive_stack_builds_from_the_offset_toward_the_normal() {
        let b = reference(0.0)
            .boundaries(&mm(), DirectionSense::Positive, -185.0, &CAVITY)
            .expect("bounds");
        close(&b.boundaries, &[-0.185, -0.07, 0.03, 0.07, 0.185]);
        assert_eq!(b.order, [0, 1, 2, 3]);
        assert_eq!(b.normal, [0.0, 1.0, 0.0]);
    }

    #[test]
    fn a_negative_stack_builds_against_the_normal_and_lists_layers_reversed() {
        let b = reference(0.0)
            .boundaries(&mm(), DirectionSense::Negative, 185.0, &CAVITY)
            .expect("bounds");
        close(&b.boundaries, &[-0.185, -0.07, -0.03, 0.07, 0.185]);
        assert_eq!(b.order, [3, 2, 1, 0]);
    }

    #[test]
    fn the_reference_distance_shifts_every_boundary() {
        let b = reference(0.1)
            .boundaries(&mm(), DirectionSense::Positive, 0.0, &[200.0])
            .expect("bounds");
        close(&b.boundaries, &[0.1, 0.3]);
    }

    #[test]
    fn a_zero_thick_layer_repeats_a_boundary() {
        let b = reference(0.0)
            .boundaries(&mm(), DirectionSense::Positive, 0.0, &[100.0, 0.0, 50.0])
            .expect("bounds");
        close(&b.boundaries, &[0.0, 0.1, 0.1, 0.15]);
    }

    #[test]
    fn unusable_inputs_are_degenerate() {
        let r = reference(0.0);
        let cases: [(f64, &[f64]); 5] = [
            (0.0, &[]),
            (f64::NAN, &[1.0]),
            (0.0, &[-1.0]),
            (0.0, &[f64::NAN]),
            (0.0, &[f64::INFINITY]),
        ];
        for (offset, thicknesses) in cases {
            let err = r
                .boundaries(&mm(), DirectionSense::Positive, offset, thicknesses)
                .expect_err("refused");
            assert!(
                matches!(&err, GeometryError::Degenerate { entity, type_name, .. }
                    if *entity == EntityId(30) && type_name == "IFCWALL"),
                "{err:?}"
            );
        }
    }
}
