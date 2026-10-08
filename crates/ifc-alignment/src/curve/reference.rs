//! The cant-carrying 3D centreline: the `IfcSegmentedReferenceCurve` role,
//! as an exact banked curve (#93).
//!
//! IFC4.3 ADD2 represents an alignment with cant as an
//! `IfcSegmentedReferenceCurve` whose base curve is the `IfcGradientCurve`.
//! The neutral value is `Curve3::Banked`: the composed centreline
//! (`Elevated3`, as [`gradient_curve3`](super::gradient_curve3) builds it)
//! with two laws over plan distance and the rail-head distance:
//!
//! - **cant** `D(d) = left - right`, one `CantPiece` per cant segment in the
//!   segment's own `xi = s / L`, exactly the base formula
//!   `IfcAlignmentCantSegmentTypeEnum` (8.7.2.1) states:
//!   `CONSTANTCANT`, `LINEARTRANSITION`, `BLOSSCURVE` `(3 - 2 xi) xi^2`,
//!   `HELMERTCURVE` as its two quadratic halves, `COSINECURVE`, `SINECURVE`,
//!   and `VIENNESEBEND` as its angle form
//!   `psi(xi) = psi1 + dpsi xi^4 (35 - 84 xi + 70 xi^2 - 20 xi^3)` with
//!   `psi = arcsin(D / b)`;
//! - **pivot** `e(d) = (left + right) / 2`: the rail heads are "measured
//!   relatively to vertical alignment" (`IfcAlignmentCantSegment`), so the
//!   section's rotation point sits midway between them, at the profile for
//!   a rotation about the centreline and at `D / 2` for one about the low
//!   rail. Each rail follows the segment's form, so their mean follows the
//!   same form with the mean values.
//!
//! # The convention: rotation about the tangent
//!
//! `Banked3` names how a cant reads on a grade, with no default. IFC4.3 ADD2
//! states cant as an angle: "the cant value D, Railhead distance b and cant
//! angle psi" are related by `psi = arcsin(D / b)`, psi being the "Angle of
//! cant (cross slope angle, bank angle)" (`IfcAlignmentCantSegmentTypeEnum`,
//! 8.7.2.1, Table 8.7.2.1.1.2.A), and the Viennese bend is written for psi
//! itself. `IfcSegmentedReferenceCurve` (8.9.3.62) carries cant by
//! interpolating the placement axes (`Axis`, `RefDirection`) between
//! segments: a rotation of the section frame. So the section is the
//! level-track section turned by `psi` about the 3D tangent,
//! `BankConvention::TangentRotation`. On a grade the rail heads then differ
//! in height by `D cos(theta)`, not `D`; the standard does not ask for the
//! other reading, and this is the rotation `CantFrame` documents.
//!
//! # Refusals that remain
//!
//! - A Viennese bend about a held rail (the low rail, say): the bend gives
//!   the bank angle, so the rotation point there is `held -+ b sin(psi) / 2`,
//!   an angle form a height-form pivot law cannot carry (Axiolid refuses it
//!   as `BankError::AngleInPivot`); refused with its own reason until
//!   `axiolid-curve` has an angle-form pivot (#364).
//!   `CantLayout::cant_at_distance` and `CantLayout::frame_at_distance`
//!   evaluate this bend, so for it, and only for it, the section frame has
//!   a value where the banked curve refuses.
//! - A Viennese bend whose pivot moves with neither rail held: the
//!   standard does not determine the rails, and `CantLayout` refuses it
//!   with the same error. A pivot that stays put through the bend, such as
//!   rotation about the centreline, lowers, and agrees with the section
//!   frame (#312).
//! - A cant layout that does not cover the plan from its start to its end:
//!   the banked curve's span is the cant law's, and a station without cant
//!   is not zero cant.
//! - `USERDEFINED`, `NOTDEFINED` and unknown cant types, which state no law.

use axiolid_curve::{bank_angle, BankConvention, Banked3, CantLaw, CantPiece, Curve3};
use axiolid_model::{GeometryGraphBuilder, GeometryNode};
use ifc_model::{EntityId, Model};

use super::assemble::{finish, LoweredAlignmentCurve};
use super::gradient::{compose, sole_layout};
use super::seam::HorizontalSeam;
use super::tolerance::SeamTolerance;
use crate::cant::{
    viennese_rotation, CantLayout, CantSegment, CantSegmentType, VienneseRotation,
    HELD_RAIL_ANGLE_PIVOT,
};
use crate::error::{AlignmentError, AlignmentResult};
use crate::horizontal::AlignmentUnits;
use crate::view::AlignmentView;

/// Lower an `IfcAlignment` with cant to its exact cant-carrying centreline,
/// a `Curve3::Banked`.
///
/// The graph holds one `Curve3::Banked` node: the centreline
/// [`lower_gradient_curve`](super::lower_gradient_curve) builds, the cant
/// and pivot laws, and the rail-head distance, under
/// `BankConvention::TangentRotation` (see the module documentation).
/// `sources` lists the horizontal, vertical and cant layouts, then the
/// vertical and cant segments; `seams` are the plan's.
///
/// # Errors
///
/// - Everything [`CantLayout::for_alignment`] refuses: a wrong entity type,
///   no `IfcAlignmentCant` or several (`SemanticViolation`), a malformed or
///   discontinuous cant layout.
/// - Everything the gradient curve refuses.
/// - [`AlignmentError::Unsupported`] for a cant layout that does not cover
///   the whole plan, a cant segment type with no law, and a Viennese bend
///   whose pivot moves.
/// - [`AlignmentError::SemanticViolation`] for a cant beyond the rail-head
///   distance (`|D| > b`) at a segment end; every form is monotone between
///   its ends, so none exceeds it inside.
pub fn lower_segmented_reference_curve(
    model: &Model,
    alignment: EntityId,
    units: AlignmentUnits,
) -> AlignmentResult<LoweredAlignmentCurve> {
    let banked = banked(model, alignment, units)?;
    let mut builder = GeometryGraphBuilder::new();
    let root = builder
        .push(GeometryNode::Curve3(Curve3::Banked(banked.curve)))
        .map_err(|error| AlignmentError::Graph {
            detail: error.to_string(),
        })?;
    let mut lowered = finish(builder, root, banked.sources)?;
    lowered.seams = banked.seams;
    Ok(lowered)
}

/// The exact cant-carrying centreline of `alignment`, an `IfcAlignment`.
///
/// The composition [`lower_segmented_reference_curve`] uses, returned
/// directly so a caller can hand it to an evaluator; this crate never
/// evaluates it.
///
/// # Errors
///
/// As [`lower_segmented_reference_curve`].
pub fn segmented_reference_curve3(
    model: &Model,
    alignment: EntityId,
    units: AlignmentUnits,
) -> AlignmentResult<Curve3> {
    banked(model, alignment, units).map(|banked| Curve3::Banked(banked.curve))
}

/// The banked curve and what the graph lowering reports with it.
struct Lowered {
    curve: Banked3,
    sources: Vec<EntityId>,
    seams: Vec<HorizontalSeam>,
}

fn banked(model: &Model, alignment: EntityId, units: AlignmentUnits) -> AlignmentResult<Lowered> {
    // Resolving the cant layout first names a missing or ambiguous one, or a
    // malformed segment, before anything about the centreline.
    let cant = CantLayout::for_alignment(model, alignment, units)?;
    let view = AlignmentView::for_model(model)?;
    let horizontal = sole_layout(&view, alignment, "IfcAlignmentHorizontal")?;
    let vertical = sole_layout(&view, alignment, "IfcAlignmentVertical")?;
    let composed = compose(model, &view, horizontal, vertical, units, Some(&cant))?;
    let tolerance = SeamTolerance::for_model(model, units)?;
    let (cant_law, pivot) = cant_laws(&cant, composed.plan_length, tolerance)?;
    if !(cant_law.is_well_formed() && pivot.is_well_formed()) {
        return Err(AlignmentError::Graph {
            detail: format!(
                "the cant layout {} did not assemble into well-formed laws",
                cant.entity
            ),
        });
    }
    let mut sources = vec![horizontal, vertical, cant.entity];
    sources.extend(composed.vertical_ids);
    sources.extend(cant.segments().iter().map(|segment| segment.entity));
    Ok(Lowered {
        curve: Banked3::new(
            composed.centreline,
            cant_law,
            pivot,
            cant.rail_head_distance,
            BankConvention::TangentRotation,
        ),
        sources,
        seams: composed.seams,
    })
}

/// The cant and pivot laws of `layout`, from plan distance zero.
///
/// The cant layout's stations are distances along the horizontal layout,
/// the distance the plan is parameterised by, so the laws need no
/// re-indexing; the layout must start at the plan start and end at its end,
/// at the model's seam tolerance. The closing zero-length segment adds no
/// piece.
fn cant_laws(
    layout: &CantLayout,
    plan_length: f64,
    tolerance: SeamTolerance,
) -> AlignmentResult<(CantLaw, CantLaw)> {
    let segments: Vec<&CantSegment> = layout
        .segments()
        .iter()
        .filter(|segment| segment.horizontal_length > 0.0)
        .collect();
    let start = segments.first().map_or(0.0, |s| s.start_dist_along);
    let end = segments
        .last()
        .map_or(0.0, |s| s.start_dist_along + s.horizontal_length);
    if !tolerance.same_length(start, 0.0) || !tolerance.same_length(end, plan_length) {
        return Err(AlignmentError::Unsupported {
            entity: layout.entity,
            type_name: "IfcAlignmentCant".to_owned(),
            detail: "the cant layout does not cover the whole plan; a banked curve's span is its \
                     cant law's, and a station without cant is not zero cant",
        });
    }
    let b = layout.rail_head_distance;
    let mut cant = Vec::with_capacity(segments.len());
    let mut pivot = Vec::with_capacity(segments.len());
    for segment in segments {
        let (left, right) = ends(segment)?;
        let d = [left[0] - right[0], left[1] - right[1]];
        let e = [0.5 * (left[0] + right[0]), 0.5 * (left[1] + right[1])];
        if d.iter().any(|value| bank_angle(*value, b).is_err()) {
            return Err(AlignmentError::SemanticViolation {
                entity: Some(segment.entity),
                rule: "cant must not exceed the rail head distance (|D| <= b)",
            });
        }
        let length = segment.horizontal_length;
        match &segment.predefined_type {
            CantSegmentType::ConstantCant => {
                cant.push(CantPiece::constant(length, d[0]));
                pivot.push(CantPiece::constant(length, e[0]));
            }
            CantSegmentType::LinearTransition => {
                cant.push(CantPiece::linear(length, d[0], d[1]));
                pivot.push(CantPiece::linear(length, e[0], e[1]));
            }
            CantSegmentType::BlossCurve => {
                cant.push(CantPiece::bloss(length, d[0], d[1]));
                pivot.push(CantPiece::bloss(length, e[0], e[1]));
            }
            CantSegmentType::HelmertCurve => {
                cant.extend(CantPiece::helmert(length, d[0], d[1]));
                pivot.extend(CantPiece::helmert(length, e[0], e[1]));
            }
            CantSegmentType::CosineCurve => {
                cant.push(CantPiece::cosine(length, d[0], d[1]));
                pivot.push(CantPiece::cosine(length, e[0], e[1]));
            }
            CantSegmentType::SineCurve => {
                cant.push(CantPiece::sine(length, d[0], d[1]));
                pivot.push(CantPiece::sine(length, e[0], e[1]));
            }
            CantSegmentType::VienneseBend => {
                // A held rail's pivot is an angle form the height-form
                // pivot law cannot carry; `cant_at` evaluates it (#364).
                match viennese_rotation(segment.entity, left, right, tolerance)? {
                    VienneseRotation::FixedPivot(_) => {}
                    VienneseRotation::HeldLeft(_) | VienneseRotation::HeldRight(_) => {
                        return Err(AlignmentError::Unsupported {
                            entity: segment.entity,
                            type_name: "VIENNESEBEND".to_owned(),
                            detail: HELD_RAIL_ANGLE_PIVOT,
                        });
                    }
                }
                // |D| <= b was checked above, so both angles exist.
                let psi = |value: f64| (value / b).asin();
                cant.push(CantPiece::viennese_bend(length, psi(d[0]), psi(d[1])));
                pivot.push(CantPiece::constant(length, e[0]));
            }
            _ => {
                return Err(AlignmentError::Unsupported {
                    entity: segment.entity,
                    type_name: segment.predefined_type_name().to_owned(),
                    detail: "cant PredefinedType has no defined base formula",
                })
            }
        }
    }
    Ok((CantLaw::new(cant), CantLaw::new(pivot)))
}

/// `[start, end]` heights of the left and right rail heads above the
/// profile. A constant cant may omit its end values; a transition needs
/// both.
fn ends(segment: &CantSegment) -> AlignmentResult<([f64; 2], [f64; 2])> {
    let constant = segment.predefined_type == CantSegmentType::ConstantCant;
    match (segment.end_cant_left, segment.end_cant_right) {
        (Some(left), Some(right)) => {
            if constant && (left != segment.start_cant_left || right != segment.start_cant_right) {
                return Err(AlignmentError::InvalidSegment {
                    entity: segment.entity,
                    detail: "CONSTANTCANT requires equal (or absent) start and end cant",
                });
            }
            Ok((
                [segment.start_cant_left, left],
                [segment.start_cant_right, right],
            ))
        }
        _ if constant => Ok(([segment.start_cant_left; 2], [segment.start_cant_right; 2])),
        _ => Err(AlignmentError::InvalidSegment {
            entity: segment.entity,
            detail: "transition cant segments require both end cant values",
        }),
    }
}
