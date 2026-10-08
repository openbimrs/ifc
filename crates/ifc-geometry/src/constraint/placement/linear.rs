//! Linear placement: where a product sits along an alignment.
//!
//! IFC4x3 places road and rail furniture by distance along a curve rather
//! than by a chain of nested boxes. The placement chain walk
//! ([`crate::constraint::local::PlacementResolver`]) reaches one here, as a
//! product's own placement or as another placement's `PlacementRelTo`
//! (#363).
//!
//! This module resolves the cached `CartesianPosition` an authoring tool
//! writes alongside the linear expression. Deriving the transform from the
//! basis curve needs a caller-supplied `CurveEvaluator` (the opt-in
//! `compile` feature, ADR 0004): with one, `derive::resolve_linear` derives
//! an uncached placement and checks a cached one (#353, #354); without one,
//! an uncached placement is refused by name, never approximated.
//!
//! # The frame a linear placement is relative to (#357)
//!
//! The station frame lies on the basis curve, and the curve's coordinates
//! are those of the product whose representation carries it. IFC4.3 ADD2
//! states which frame that is twice over:
//!
//! - concept Product Linear Placement: "the IfcLinearPositioningElement.
//!   ObjectPlacement sets the context for all elements positioned on it.
//!   Consequently, each product placement that uses Product Linear
//!   Placement references the IfcObjectPlacement of the
//!   IfcLinearPositioningElement through IfcLinearPlacement.PlacementRelTo";
//! - `IfcObjectPlacement.PlacementRelTo`: "If it is omitted, then in the
//!   case of linear placement it is established by the origin of horizontal
//!   alignment of the referenced IfcAlignment Axis."
//!
//! So the frame is `PlacementRelTo` when stated, and otherwise the
//! `ObjectPlacement` of the alignment whose representation carries the
//! basis curve (an `IfcAlignment` basis is that alignment). We read "the
//! origin of horizontal alignment of the referenced IfcAlignment Axis" as
//! the origin of the coordinate system that `Axis` representation is
//! stated in, which is the alignment's object coordinate system: reading it
//! as the curve's start point would translate every station by the start
//! point a second time, since the curve's own coordinates already contain
//! it. When both are present and resolve to different frames the
//! placement is refused ([`GeometryError::PlacementRelToConflict`]); a curve
//! that no product carries, with no `PlacementRelTo`, is read in the
//! representation context's coordinate system, as an `IfcLocalPlacement`
//! without `PlacementRelTo` is ("In the case of local placement it is
//! established by the geometric representation context").
//!
//! `CartesianPosition` is, in the IFC4.x development documentation's words,
//! an "Optional fallback for the `RelativePlacement` attribute, which may be
//! used by importing applications that do not support linear placement";
//! IFC4.3 ADD2 itself gives it no description. A fallback for
//! `RelativePlacement` stands where `RelativePlacement` stands, so it is
//! relative to the same frame, exactly as an `IfcLocalPlacement`'s
//! `RelativePlacement` is relative to its `PlacementRelTo`. That keeps the
//! cache and the derivation comparable (#354): both compose with the same
//! parent.
//!
//! The representation context's `WorldCoordinateSystem` is not composed
//! here. It sits above every placement chain -- the alignment's and the
//! product's alike -- and representation frames apply it once
//! (`input::context::representation_frame`). Composing it into the chain
//! as well would apply it twice.
//!
//! ADR 0003 (amended 2026-09-15) permits the `ifc-alignment` dependency:
//! bridges may depend on bridges.

use ifc_model::{EntityId, Model, Value};

use super::LinearResolution;
use crate::error::{GeometryError, GeometryResult};
use crate::input::product::Product;
use crate::resource::placement::axis_placement_transform;
use crate::transform::Transform;
use crate::units::UnitScale;

/// `IfcLinearPlacement` attribute slots.
///
/// IFC4X3_ADD2: inherited `PlacementRelTo` is slot 0; `RelativePlacement`
/// and `CartesianPosition` are this declaration own slots 1..2.
mod slot {
    /// `PlacementRelTo`: the frame the placement is relative to, optional.
    pub const PLACEMENT_REL_TO: usize = 0;
    /// `CartesianPosition`: the cached explicit placement, optional.
    pub const CARTESIAN_POSITION: usize = 2;
}

/// `IfcProduct.Representation`, `IfcProductDefinitionShape.Representations`
/// and `IfcRepresentation.Items`: the same slots in IFC2X3, IFC4 and IFC4X3.
mod representation_slot {
    pub const PRODUCT_REPRESENTATION: usize = 6;
    pub const REPRESENTATIONS: usize = 2;
    pub const ITEMS: usize = 3;
}

/// The positioning-element types whose `Representation` carries an
/// alignment's curves (IFC4X3 ADD2 `IfcLinearPositioningElement` and its
/// only subtype).
const ALIGNMENT_TYPES: [&str; 2] = ["IFCALIGNMENT", "IFCLINEARPOSITIONINGELEMENT"];

/// The stated `PlacementRelTo` of an `IfcLinearPlacement`.
pub(crate) fn placement_rel_to(model: &Model, placement: EntityId) -> Option<EntityId> {
    model
        .get(placement)?
        .attributes
        .get(slot::PLACEMENT_REL_TO)
        .and_then(Value::as_ref_id)
}

/// Validate the linear expression and return its basis curve.
///
/// A file whose expression is malformed is broken whether or not a cached
/// shortcut happens to be present, so this runs on every path.
pub(crate) fn basis_curve(
    model: &Model,
    units: &UnitScale,
    placement: EntityId,
) -> GeometryResult<EntityId> {
    let file_units = ifc_alignment::AlignmentUnits {
        length_to_metres: 1.0,
        angle_to_radians: units.angle_to_radians,
    };
    ifc_alignment::resolve_linear_placement(model, placement, file_units)
        .map(|resolved| resolved.relative_placement.basis_curve)
        .map_err(|_error| GeometryError::Unsupported {
            entity: placement,
            type_name: "IFCLINEARPLACEMENT".into(),
            detail: "linear placement is malformed; ifc-alignment refused it",
        })
}

/// The frame a basis curve's coordinates are stated in, as far as the
/// model says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CurveFrame {
    /// Carried by a product placed by this `ObjectPlacement`.
    Placed {
        /// The product carrying the curve.
        product: EntityId,
        /// Its `ObjectPlacement`.
        placement: EntityId,
    },
    /// Carried by a product without an `ObjectPlacement`: model space.
    ModelSpace {
        /// The product carrying the curve.
        product: EntityId,
    },
    /// No product carries the curve.
    Unknown,
}

/// The frame `curve` is stated in.
///
/// An alignment (`IfcAlignment`, `IfcLinearPositioningElement`) is looked up
/// first: it is the carrier IFC names, and there are few of them. Only when
/// `thorough` is set -- no `PlacementRelTo` was stated, so the frame has to
/// come from somewhere -- are the other products searched, which costs a
/// scan of the model.
///
/// # Errors
///
/// A curve carried by several products placed differently is
/// [`GeometryError::Unsupported`] naming the curve: its frame is ambiguous.
pub(crate) fn curve_frame(
    model: &Model,
    curve: EntityId,
    thorough: bool,
) -> GeometryResult<CurveFrame> {
    let mut carriers = Vec::new();
    if model
        .get(curve)
        .is_some_and(|entity| ALIGNMENT_TYPES.iter().any(|t| entity.is_type(t)))
    {
        carriers.push(curve);
    }
    if carriers.is_empty() {
        for type_name in ALIGNMENT_TYPES {
            for &product in model.ids_of_type(type_name) {
                if carries(model, product, curve) {
                    carriers.push(product);
                }
            }
        }
    }
    if carriers.is_empty() && thorough {
        carriers = carriers_by_scan(model, curve);
    }
    let mut frames = carriers.into_iter().map(|product| {
        let placement = model
            .get(product)
            .and_then(|entity| Product::new(product, entity).object_placement());
        match placement {
            Some(placement) => CurveFrame::Placed { product, placement },
            None => CurveFrame::ModelSpace { product },
        }
    });
    let Some(first) = frames.next() else {
        return Ok(CurveFrame::Unknown);
    };
    let same_frame = |a: &CurveFrame, b: &CurveFrame| match (a, b) {
        (CurveFrame::Placed { placement: x, .. }, CurveFrame::Placed { placement: y, .. }) => {
            x == y
        }
        (CurveFrame::ModelSpace { .. }, CurveFrame::ModelSpace { .. }) => true,
        _ => false,
    };
    for other in frames {
        if !same_frame(&first, &other) {
            return Err(GeometryError::Unsupported {
                entity: curve,
                type_name: model
                    .get(curve)
                    .map_or_else(String::new, |entity| entity.type_name.to_string()),
                detail: "the basis curve is carried by several products placed differently, so \
                         the frame it is stated in is ambiguous; state PlacementRelTo",
            });
        }
    }
    Ok(first)
}

/// Does `product`'s representation list `curve` as an item?
fn carries(model: &Model, product: EntityId, curve: EntityId) -> bool {
    let refs = |entity: EntityId, slot: usize| -> Vec<EntityId> {
        match model.get(entity).and_then(|e| e.attributes.get(slot)) {
            Some(Value::List(items)) => items.iter().filter_map(Value::as_ref_id).collect(),
            Some(value) => value.as_ref_id().into_iter().collect(),
            None => Vec::new(),
        }
    };
    refs(product, representation_slot::PRODUCT_REPRESENTATION)
        .into_iter()
        .flat_map(|shape| refs(shape, representation_slot::REPRESENTATIONS))
        .any(|representation| refs(representation, representation_slot::ITEMS).contains(&curve))
}

/// Every product whose representation lists `curve`, by scanning.
fn carriers_by_scan(model: &Model, curve: EntityId) -> Vec<EntityId> {
    let lists = |entity: &ifc_model::Entity, slot: usize, target: EntityId| {
        matches!(entity.attributes.get(slot), Some(Value::List(items))
            if items.iter().any(|item| item.as_ref_id() == Some(target)))
    };
    let representations: Vec<EntityId> = model
        .of_type("IFCSHAPEREPRESENTATION")
        .filter(|(_, entity)| lists(entity, representation_slot::ITEMS, curve))
        .map(|(id, _)| id)
        .collect();
    if representations.is_empty() {
        return Vec::new();
    }
    let shapes: Vec<EntityId> = model
        .of_type("IFCPRODUCTDEFINITIONSHAPE")
        .filter(|(_, entity)| {
            representations
                .iter()
                .any(|&r| lists(entity, representation_slot::REPRESENTATIONS, r))
        })
        .map(|(id, _)| id)
        .collect();
    model
        .iter()
        .filter(|(_, entity)| {
            entity
                .attributes
                .get(representation_slot::PRODUCT_REPRESENTATION)
                .and_then(Value::as_ref_id)
                .is_some_and(|shape| shapes.contains(&shape))
        })
        .map(|(id, _)| id)
        .collect()
}

/// An `IfcLinearPlacement`'s own transform, in metres, relative to the
/// frame its basis curve is stated in (module documentation).
///
/// `parent` is that frame's world transform in metres; only the cache
/// check reads it, to report world positions.
///
/// # Errors
///
/// Without an evaluator in `linear`, refuses when the placement states only
/// the linear expression. Computing a frame from the basis curve needs
/// curve evaluation at a distance, which this crate never does itself: a
/// wrong sign on the offset puts a sign on the wrong side of a carriageway,
/// so a typed refusal names the evaluator-taking entry points instead.
pub(crate) fn relative_transform(
    model: &Model,
    units: &UnitScale,
    placement: EntityId,
    linear: LinearResolution<'_>,
    parent: &Transform,
) -> GeometryResult<Transform> {
    let entity = model.get(placement).ok_or(GeometryError::MissingEntity {
        referrer: placement,
        missing: placement,
    })?;
    let cached = entity
        .attributes
        .get(slot::CARTESIAN_POSITION)
        .and_then(Value::as_ref_id);

    #[cfg(feature = "compile")]
    if let Some(derivation) = linear.derivation {
        let resolved = super::derive::resolve_in_file_units(model, units, placement)?;
        let cached = cached
            .map(|cartesian| cached_transform(model, units, placement, cartesian))
            .transpose()?;
        return super::derive::resolve_linear(
            model, units, placement, &resolved, cached, derivation, parent,
        );
    }
    #[cfg(not(feature = "compile"))]
    let _ = (linear, parent);

    let Some(cartesian) = cached else {
        // Without the cached shortcut the frame has to be derived by
        // evaluating the basis curve, which is computation and therefore
        // opt-in under ADR 0004. Point the caller at the capability rather
        // than approximating a position here.
        return Err(GeometryError::Unsupported {
            entity: placement,
            type_name: "IFCLINEARPLACEMENT".into(),
            detail: "placement states only IfcPointByDistanceExpression and no CartesianPosition; \
                     derive the frame with a CurveEvaluator (feature `compile`: \
                     LoweringSession::with_curve_evaluator or \
                     product_world_transform_with_evaluator)",
        });
    };
    cached_transform(model, units, placement, cartesian)
}

/// The cached `CartesianPosition` as a transform, in metres.
fn cached_transform(
    model: &Model,
    units: &UnitScale,
    placement: EntityId,
    cartesian: EntityId,
) -> GeometryResult<Transform> {
    let cartesian_entity = model.get(cartesian).ok_or(GeometryError::MissingEntity {
        referrer: placement,
        missing: cartesian,
    })?;
    Ok(axis_placement_transform(model, cartesian, cartesian_entity)?.to_metres(units))
}
