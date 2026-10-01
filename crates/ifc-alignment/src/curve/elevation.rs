//! Vertical segments as exact elevation laws.
//!
//! A vertical segment states height as a function of distance along the
//! horizontal layout, not along the 3D curve. The two differ by
//! sqrt(1 + g^2) wherever the grade is non-zero, so the distinction is kept
//! explicit here rather than left to a caller to rediscover.
//!
//! `ElevationLaw::parabolic` encodes
//! `z(d) = height + entry * d + (exit - entry) / (2 * length) * d^2`, which
//! is the IFC parabolic vertical curve exactly, so PARABOLICARC needs no
//! approximation.
//!
//! # Circular arcs and clothoids are a recorded refusal (#91, #258)
//!
//! IFC4.3 ADD2 (`IfcAlignmentVerticalSegmentTypeEnum`) defines
//! `CIRCULARARC` as "the derivative of vertical angle with respect to
//! sloping length along the track (3D length) is constant" and maps it to an
//! `IfcCircle` parent curve in the (distance along, height) plane, and
//! `CLOTHOID` as a vertical curvature varying linearly in that 3D length.
//! Neither height is a polynomial in plan distance: the arc is
//! `z = z_c - sqrt(R^2 - (d - d_c)^2)` and the clothoid is a Fresnel
//! integral. The standard quotes the EN 13803 ordinate
//! `z_c(s) = s^2 / (2 R)` only as the offset from the tangent, not as the
//! definition of the segment, so substituting a parabola would move the road
//! surface. The pinned `ElevationLaw` has only polynomial pieces, so both
//! are typed [`AlignmentError::Unsupported`] refusals until Axiolid has an
//! exact law for them (the upstream request is recorded on #258).

use axiolid_curve::ElevationLaw;
use ifc_model::{EntityId, Model};

use super::tolerance::SeamTolerance;
use crate::error::{AlignmentError, AlignmentResult, ProfileSeam};
use crate::horizontal::AlignmentUnits;
use crate::vertical::{read_vertical_segment, VerticalSegment, VerticalSegmentType};
use crate::view::AlignmentView;

/// The exact elevation law for one `IfcAlignmentVerticalSegment`.
///
/// The law is written in the segment own distance, restarting at zero, so a
/// segment can be moved along the profile without rewriting its
/// coefficients.
///
/// # Errors
///
/// Refuses a segment whose family has no exact polynomial form
/// (`CIRCULARARC`, `CLOTHOID`), and one whose stated parameters contradict
/// its family.
pub fn elevation_law(segment: &VerticalSegment) -> AlignmentResult<ElevationLaw> {
    match segment.predefined_type {
        VerticalSegmentType::ConstantGradient => constant_gradient(segment),
        VerticalSegmentType::ParabolicArc => parabolic_arc(segment),
        ref kind => Err(AlignmentError::Unsupported {
            entity: segment.entity,
            type_name: kind.source_name().to_owned(),
            detail: refusal(kind),
        }),
    }
}

/// Why a vertical family has no exact elevation law, by name.
fn refusal(kind: &VerticalSegmentType) -> &'static str {
    match kind {
        // A circle in (distance, height): z = z_c - sqrt(R^2 - (d - d_c)^2).
        VerticalSegmentType::CircularArc => {
            "no exact elevation law: a vertical circular arc is not polynomial in plan distance, \
             and the pinned ElevationLaw has only polynomial pieces"
        }
        // Curvature linear in 3D arc length: a Fresnel integral.
        VerticalSegmentType::Clothoid => {
            "no exact elevation law: a vertical clothoid is a Fresnel integral in plan distance, \
             and the pinned ElevationLaw has only polynomial pieces"
        }
        _ => "no exact elevation law: the vertical PredefinedType defines no curve law",
    }
}

/// `CONSTANTGRADIENT`: a straight grade, degree 1.
///
/// The two gradients are compared at rounding precision, not bit for bit:
/// an exporter that writes `0.0424413181578388` and `0.0424413181578387`
/// (Trimble, IFC4.x-IF) states one grade. The law uses `StartGradient`. A
/// zero `RadiusOfCurvature` is the straight-line convention, not a curve.
fn constant_gradient(segment: &VerticalSegment) -> AlignmentResult<ElevationLaw> {
    let curved = segment.radius_of_curvature.is_some_and(|r| r != 0.0);
    if curved
        || !SeamTolerance::strict().same_gradient(segment.start_gradient, segment.end_gradient)
    {
        return Err(AlignmentError::InvalidSegment {
            entity: segment.entity,
            detail: "CONSTANTGRADIENT requires equal gradients and no non-zero curvature radius",
        });
    }
    Ok(ElevationLaw::constant_grade(
        segment.start_height,
        segment.start_gradient,
    ))
}

/// `PARABOLICARC`: the vertical curve joining two grades, degree 2.
///
/// `ElevationLaw::parabolic` divides by the length, and documents that a
/// non-positive length is storable because naming it is a validator job.
/// This is that validator: a zero-length parabola would otherwise produce an
/// infinite or NaN coefficient and place the road surface nowhere.
///
/// The length check and the well-formedness check below overlap: removing
/// either alone still refuses a zero length, because the infinity it
/// produces is caught by the other. Both are kept deliberately. The length
/// check names the actual fault, where `is_well_formed` would only report a
/// non-finite coefficient, and it keeps holding if the kernel ever makes the
/// division total.
fn parabolic_arc(segment: &VerticalSegment) -> AlignmentResult<ElevationLaw> {
    if segment.horizontal_length <= 0.0 {
        return Err(AlignmentError::InvalidSegment {
            entity: segment.entity,
            detail: "PARABOLICARC requires a positive horizontal length",
        });
    }
    let law = ElevationLaw::parabolic(
        segment.start_height,
        segment.start_gradient,
        segment.end_gradient,
        segment.horizontal_length,
    );
    // The guard above rules out the division blowing up, but the authored
    // heights and grades are still file data.
    if !law.is_well_formed() {
        return Err(AlignmentError::InvalidSegment {
            entity: segment.entity,
            detail: "PARABOLICARC parameters do not form a finite elevation law",
        });
    }
    Ok(law)
}

/// The whole vertical profile as one law over distance along the plan.
///
/// Segments are authored as a run, each with its own `StartDistAlong`, and
/// `ElevationLaw::Piecewise` wants interior seams plus one law per piece,
/// each written in its own distance restarting at zero. That is exactly how
/// [`elevation_law`] writes a segment, so no rebasing is needed here.
///
/// Every segment restates where it starts: `StartHeight` and
/// `StartGradient`. Those must agree with where the previous segment ends --
/// its law's height at its own `HorizontalLength`, and its `EndGradient` --
/// or the profile has a step or a kink at the seam. Joining it anyway would
/// silently shift every downstream height (a step) or its slope (a kink),
/// so both are refused with [`AlignmentError::ProfileDiscontinuity`].
///
/// Seams are compared with [`SeamTolerance::strict`]: floating-point
/// rounding only. A file whose exporter rounds stations or heights states
/// that in its `Precision`; use [`profile_law_within`] with
/// [`SeamTolerance::for_model`], or [`vertical_profile_law`], to honour it.
///
/// # Errors
///
/// Refuses an empty profile, segments that are not sorted and contiguous,
/// a height or grade discontinuity at any seam, and any segment without an
/// exact law.
pub fn profile_law(segments: &[VerticalSegment]) -> AlignmentResult<ElevationLaw> {
    profile_law_within(segments, SeamTolerance::strict())
}

/// [`profile_law`] with an explicit seam tolerance.
///
/// `tolerance` widens only the LENGTH seams (`StartDistAlong` contiguity and
/// `StartHeight`); gradient seams stay at rounding precision. See
/// [`SeamTolerance`] for the rule and its evidence.
///
/// # Errors
///
/// As [`profile_law`].
pub fn profile_law_within(
    segments: &[VerticalSegment],
    tolerance: SeamTolerance,
) -> AlignmentResult<ElevationLaw> {
    let Some(first) = segments.first() else {
        return Err(AlignmentError::SemanticViolation {
            entity: None,
            rule: "a vertical profile must have at least one segment",
        });
    };
    if segments.len() == 1 {
        return elevation_law(first);
    }

    let mut laws: Vec<ElevationLaw> = Vec::with_capacity(segments.len());
    let mut breaks = Vec::with_capacity(segments.len() - 1);
    let start = first.start_dist_along;
    for (index, segment) in segments.iter().enumerate() {
        let law = elevation_law(segment)?;
        if let (Some(previous), Some(previous_law)) =
            (index.checked_sub(1).map(|i| &segments[i]), laws.last())
        {
            let expected = previous.start_dist_along + previous.horizontal_length;
            // A gap or overlap means the profile does not describe one
            // continuous road. Joining it anyway would silently move every
            // downstream height, so it is refused.
            if !tolerance.same_length(segment.start_dist_along, expected) {
                return Err(AlignmentError::InvalidSegment {
                    entity: segment.entity,
                    detail: "vertical segments must be contiguous and ascending in StartDistAlong",
                });
            }
            check_seam(previous, previous_law, segment, tolerance)?;
            breaks.push(segment.start_dist_along - start);
        }
        laws.push(law);
    }
    Ok(ElevationLaw::Piecewise { breaks, laws })
}

/// Refuse a step in height or a kink in grade where `segment` begins.
///
/// The previous end height is evaluated from its exact law rather than
/// recomputed here, so the seam is compared against the same polynomial the
/// profile will evaluate. The previous end grade is its authored
/// `EndGradient`; for a constant gradient that equals `StartGradient`,
/// which `elevation_law` has already enforced.
fn check_seam(
    previous: &VerticalSegment,
    previous_law: &ElevationLaw,
    segment: &VerticalSegment,
    tolerance: SeamTolerance,
) -> AlignmentResult<()> {
    let end_height = previous_law.height_at(previous.horizontal_length).ok_or(
        AlignmentError::InvalidSegment {
            entity: previous.entity,
            detail: "vertical segment has no finite end height",
        },
    )?;
    let seams = [
        (
            ProfileSeam::Height,
            end_height,
            segment.start_height,
            tolerance.same_length(segment.start_height, end_height),
        ),
        (
            ProfileSeam::Gradient,
            previous.end_gradient,
            segment.start_gradient,
            tolerance.same_gradient(segment.start_gradient, previous.end_gradient),
        ),
    ];
    for (seam, expected, actual, same) in seams {
        if !same {
            return Err(AlignmentError::ProfileDiscontinuity {
                entity: segment.entity,
                previous: previous.entity,
                seam,
                expected,
                actual,
            });
        }
    }
    Ok(())
}

/// The exact profile of an `IfcAlignmentVertical`, checked at the seam
/// tolerance the model declares.
///
/// Reads the nested segment chain in authored order and joins it with
/// [`profile_law_within`] at [`SeamTolerance::for_model`]. This is the
/// entry point for a profile read from a file: an exporter that rounds
/// stations or heights to its stated `Precision` is accepted, a step beyond
/// it is still refused.
///
/// Like [`profile_law`], the law is indexed by distance from the FIRST
/// segment's `StartDistAlong`; the composed gradient curve re-indexes it to
/// plan distance.
///
/// # Errors
///
/// Refuses a model that is not IFC4X3, an entity that is not an
/// `IfcAlignmentVertical`, an empty layout, an invalid declared
/// `Precision`, and everything [`profile_law`] refuses.
pub fn vertical_profile_law(
    model: &Model,
    entity: EntityId,
    units: AlignmentUnits,
) -> AlignmentResult<ElevationLaw> {
    let view = AlignmentView::for_model(model)?;
    let layout = model
        .get(entity)
        .ok_or(AlignmentError::MissingEntity { entity })?;
    if !view.schema.is_a(&layout.type_name, "IfcAlignmentVertical") {
        return Err(AlignmentError::WrongType {
            entity,
            expected: "IfcAlignmentVertical",
            actual: layout.type_name.to_string(),
        });
    }
    let tolerance = SeamTolerance::for_model(model, units)?;
    let segments = view
        .segment_chain(entity, "IfcAlignmentVerticalSegment")?
        .into_iter()
        .map(|id| read_vertical_segment(model, id, units))
        .collect::<AlignmentResult<Vec<_>>>()?;
    if segments.is_empty() {
        return Err(AlignmentError::SemanticViolation {
            entity: Some(entity),
            rule: "IfcAlignmentVertical must nest at least one IfcAlignmentSegment",
        });
    }
    profile_law_within(&segments, tolerance)
}

/// Re-index a profile law from its first `StartDistAlong` to plan distance.
///
/// IFC4.3 ADD2 measures `IfcAlignmentVerticalSegment.StartDistAlong` "from
/// the start point of `IfcAlignmentHorizontal`", and `Elevated3` reads its
/// law at plan distance. [`profile_law`] writes the law from the profile's
/// own start, so a profile starting at station `start` must be shifted by it
/// before composition, or every height lands `start` metres early.
///
/// - `start` within `tolerance` of zero: the law is already plan-indexed.
/// - `start < 0` (the profile begins before the plan): pieces wholly before
///   the plan start are dropped and the piece straddling it is rewritten by
///   an exact Taylor shift, `q(d) = p(d + o)`, so plan distance 0 reads the
///   profile at station 0.
/// - `start > 0`, or a profile ending (`end`, its last station) before the
///   plan starts: refused. Heights before the first `StartDistAlong` do not
///   exist, and `Elevated3` has no domain bound to exclude them; composing
///   anyway would invent the surface over `0..start`.
pub(crate) fn indexed_from_plan_start(
    law: ElevationLaw,
    start: f64,
    end: f64,
    vertical: EntityId,
    tolerance: SeamTolerance,
) -> AlignmentResult<ElevationLaw> {
    if tolerance.same_length(start, 0.0) {
        return Ok(law);
    }
    if start > 0.0 || end < 0.0 {
        return Err(AlignmentError::Unsupported {
            entity: vertical,
            type_name: "IfcAlignmentVertical".to_owned(),
            detail: "the vertical profile does not cover the plan start; the composed curve has \
                     no domain to leave stations outside the profile without heights",
        });
    }
    let offset = -start;
    let malformed = || AlignmentError::InvalidSegment {
        entity: vertical,
        detail: "vertical profile law is not a run of polynomial pieces",
    };
    let (breaks, laws) = match law {
        ElevationLaw::Piecewise { breaks, laws } => (breaks, laws),
        single => (Vec::new(), vec![single]),
    };
    // The piece holding profile distance `offset`, as `partition_point` in
    // the kernel's own `piece_at` picks it: a seam belongs to the piece
    // starting there.
    let index = breaks.partition_point(|b| *b <= offset);
    let piece_start = if index == 0 { 0.0 } else { breaks[index - 1] };
    let mut laws = laws.into_iter().skip(index);
    let Some(ElevationLaw::Polynomial { coefficients }) = laws.next() else {
        return Err(malformed());
    };
    let mut shifted = vec![taylor_shift(&coefficients, offset - piece_start)];
    shifted.extend(laws);
    Ok(match shifted.len() {
        1 => shifted.remove(0),
        _ => ElevationLaw::Piecewise {
            breaks: breaks[index..].iter().map(|b| b - offset).collect(),
            laws: shifted,
        },
    })
}

/// Coefficients of `q(d) = p(d + offset)`, exactly by the binomial theorem:
/// `q_k = sum_{j >= k} C(j, k) p_j offset^(j - k)`.
fn taylor_shift(coefficients: &[f64], offset: f64) -> ElevationLaw {
    let shifted = (0..coefficients.len())
        .map(|k| {
            let mut binomial = 1.0;
            let mut power = 1.0;
            let mut sum = 0.0;
            for (j, coefficient) in coefficients.iter().enumerate().skip(k) {
                if j > k {
                    binomial = binomial * j as f64 / (j - k) as f64;
                    power *= offset;
                }
                sum += binomial * coefficient * power;
            }
            sum
        })
        .collect();
    ElevationLaw::Polynomial {
        coefficients: shifted,
    }
}
