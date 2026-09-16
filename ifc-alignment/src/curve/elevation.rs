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

use axiolid_curve::ElevationLaw;

use crate::error::{AlignmentError, AlignmentResult};
use crate::vertical::{VerticalSegment, VerticalSegmentType};

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
        // A circular vertical curve is a circle, not a polynomial in plan
        // distance: z = c - sqrt(r^2 - d^2) has no exact ElevationLaw form.
        // Road profiles overwhelmingly use the parabola, and quietly
        // substituting one for the other would move the road surface, so
        // this is refused by name.
        ref kind => Err(AlignmentError::Unsupported {
            entity: segment.entity,
            type_name: kind.source_name().to_owned(),
            detail: "no exact elevation law: only constant gradient and parabolic arc are polynomial in plan distance",
        }),
    }
}

/// `CONSTANTGRADIENT`: a straight grade, degree 1.
fn constant_gradient(segment: &VerticalSegment) -> AlignmentResult<ElevationLaw> {
    if segment.radius_of_curvature.is_some() || segment.start_gradient != segment.end_gradient {
        return Err(AlignmentError::InvalidSegment {
            entity: segment.entity,
            detail: "CONSTANTGRADIENT requires equal gradients and no curvature radius",
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
/// # Errors
///
/// Refuses an empty profile, segments that are not sorted and contiguous,
/// and any segment without an exact law.
pub fn profile_law(segments: &[VerticalSegment]) -> AlignmentResult<ElevationLaw> {
    let Some(first) = segments.first() else {
        return Err(AlignmentError::SemanticViolation {
            entity: None,
            rule: "a vertical profile must have at least one segment",
        });
    };
    if segments.len() == 1 {
        return elevation_law(first);
    }

    let mut laws = Vec::with_capacity(segments.len());
    let mut breaks = Vec::with_capacity(segments.len() - 1);
    let start = first.start_dist_along;
    for (index, segment) in segments.iter().enumerate() {
        laws.push(elevation_law(segment)?);
        if index > 0 {
            let previous = &segments[index - 1];
            let expected = previous.start_dist_along + previous.horizontal_length;
            // A gap or overlap means the profile does not describe one
            // continuous road. Joining it anyway would silently move every
            // downstream height, so it is refused.
            if !approximately(segment.start_dist_along, expected) {
                return Err(AlignmentError::InvalidSegment {
                    entity: segment.entity,
                    detail: "vertical segments must be contiguous and ascending in StartDistAlong",
                });
            }
            breaks.push(segment.start_dist_along - start);
        }
    }
    Ok(ElevationLaw::Piecewise { breaks, laws })
}

/// Equal within a tolerance scaled to the magnitude involved.
///
/// Station values run to tens of thousands of metres, where an exact
/// equality test on f64 would reject a profile that is contiguous to any
/// meaning a surveyor would recognise.
fn approximately(left: f64, right: f64) -> bool {
    let scale = left.abs().max(right.abs()).max(1.0);
    (left - right).abs() <= 1e-9 * scale
}
