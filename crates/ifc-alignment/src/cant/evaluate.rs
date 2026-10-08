//! Exact closed-form cant variation `D(s)` for each defined segment type.
//!
//! Every type in `IfcAlignmentCantSegmentTypeEnum` (buildingSMART IFC4X3
//! §8.7.2.1 base formulas) states cant directly as a value `D(ξ)` of the
//! normalized position `ξ = s / L`, not as an integral of a curvature
//! function. That is the crucial difference from the horizontal/vertical
//! transition families: cant transition never requires Fresnel-type
//! integration, so every defined type here is exactly representable.
//!
//! `VIENNESEBEND` is the one type that routes through an angle. IFC4.3 ADD2
//! (`IfcAlignmentCantSegmentTypeEnum`, 8.7.2.1) writes it for the cant
//! angle of the whole section: `ψ = arcsin(D / b)`, with `D` "the amount by
//! which one running rail is raised above the other" (so `D = left - right`)
//! and `b` the rail-head distance, blended as
//! `ψ(ξ) = ψ1 + Δψ · ξ⁴ · (35 − 84ξ + 70ξ² − 20ξ³)`. `D = b · sin(ψ)` then
//! follows exactly -- `sin` is an elementary function, not a transcendental
//! integral. The law states the cant, not either rail, so the rail heights
//! need a rule (#312):
//!
//! - **a pivot that stays put**: the section's rotation point
//!   `e = (left + right) / 2` is the same at both ends (rotation about the
//!   centreline, or about any fixed height), so the rails are `e ± D / 2`;
//! - **a pivot that moves** (rotation about the low rail, say) would follow
//!   `b sin(ψ) / 2`, an angle form the standard does not state as a law of
//!   its own: refused inside the segment with
//!   [`AlignmentError::Unsupported`], exactly as the banked centreline
//!   (`lower_segmented_reference_curve`) refuses it. At `ξ = 0` and `ξ = 1`
//!   the rails are the authored `StartCant*` / `EndCant*` values, which
//!   need no law.

use crate::cant::segment::{CantSegment, CantSegmentType};
use crate::curve::SeamTolerance;
use crate::error::{AlignmentError, AlignmentResult};

/// Why a Viennese bend whose rotation point moves is refused: the cant
/// evaluation and the banked lowering give the same reason.
pub(crate) const MOVING_VIENNESE_PIVOT: &str =
    "the section's rotation point moves through a Viennese bend: the bend gives the bank \
     angle, so a pivot such as the low rail follows b sin(psi) / 2, an angle form a \
     height-form pivot law cannot carry (Axiolid BankError::AngleInPivot)";

/// Cant applied to each rail at one normalized position along a segment.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct CantAtStation {
    /// Elevation of the left rail above the reference plane.
    pub left: f64,
    /// Elevation of the right rail above the reference plane.
    pub right: f64,
}

/// Evaluate a cant segment's exact `D(ξ)` for both rails.
///
/// `xi` is the normalized position along the segment, `0.0` at
/// `StartDistAlong` and `1.0` at `StartDistAlong + HorizontalLength`, matching
/// the spec's own `ξ = s / L` parameterization.
///
/// `rail_head_distance` is the parent `IfcAlignmentCant.RailHeadDistance`;
/// only `VIENNESEBEND` uses it, but every caller passes the parent's declared
/// value so a caller building a chain cannot forget it for the one type that
/// needs it.
///
/// A `VIENNESEBEND` is evaluated for the section's cant `D = left - right`
/// and its rails placed about a rotation point that stays put (see the
/// module documentation). Here the two ends' rotation points must agree to
/// floating-point rounding ([`SeamTolerance::strict`]);
/// [`CantLayout::cant_at_distance`](crate::cant::CantLayout::cant_at_distance)
/// compares them at the model's declared precision, as the banked lowering
/// does.
///
/// # Errors
///
/// - [`AlignmentError::InvalidSegment`] for `xi` outside `[0, 1]`, missing
///   end values on a transition, a `CONSTANTCANT` whose ends disagree, a
///   `VIENNESEBEND` without a rail-head distance or with `|D| > b` at an end;
/// - [`AlignmentError::InvalidUnits`] for a non-positive rail-head distance;
/// - [`AlignmentError::Unsupported`] for a type with no base formula, and
///   inside a `VIENNESEBEND` whose rotation point moves.
pub fn cant_at(
    segment: &CantSegment,
    xi: f64,
    rail_head_distance: Option<f64>,
) -> AlignmentResult<CantAtStation> {
    cant_within(segment, xi, rail_head_distance, SeamTolerance::strict())
}

/// [`cant_at`], comparing a Viennese bend's rotation points at `tolerance`.
pub(crate) fn cant_within(
    segment: &CantSegment,
    xi: f64,
    rail_head_distance: Option<f64>,
    tolerance: SeamTolerance,
) -> AlignmentResult<CantAtStation> {
    if !(0.0..=1.0).contains(&xi) || !xi.is_finite() {
        return Err(AlignmentError::InvalidSegment {
            entity: segment.entity,
            detail: "normalized position must lie in [0.0, 1.0]",
        });
    }

    match &segment.predefined_type {
        CantSegmentType::ConstantCant => {
            if segment
                .end_cant_left
                .is_some_and(|end| end != segment.start_cant_left)
                || segment
                    .end_cant_right
                    .is_some_and(|end| end != segment.start_cant_right)
            {
                return Err(AlignmentError::InvalidSegment {
                    entity: segment.entity,
                    detail: "CONSTANTCANT requires equal (or absent) start and end cant",
                });
            }
            Ok(CantAtStation {
                left: segment.start_cant_left,
                right: segment.start_cant_right,
            })
        }
        CantSegmentType::VienneseBend => {
            let b = rail_head_distance.ok_or(AlignmentError::InvalidSegment {
                entity: segment.entity,
                detail: "VIENNESEBEND requires the parent IfcAlignmentCant.RailHeadDistance",
            })?;
            if !(b.is_finite() && b > 0.0) {
                return Err(AlignmentError::InvalidUnits {
                    detail: "RailHeadDistance must be finite and positive",
                });
            }
            vienna_bend(segment, xi, b, tolerance)
        }
        other => {
            let fraction =
                shape_fraction(other, xi).ok_or_else(|| AlignmentError::Unsupported {
                    entity: segment.entity,
                    type_name: segment.predefined_type_name().to_owned(),
                    detail: "cant PredefinedType has no defined base formula",
                })?;
            let (end_left, end_right) = required_ends(segment)?;
            Ok(CantAtStation {
                left: segment.start_cant_left + fraction * (end_left - segment.start_cant_left),
                right: segment.start_cant_right + fraction * (end_right - segment.start_cant_right),
            })
        }
    }
}

fn required_ends(segment: &CantSegment) -> AlignmentResult<(f64, f64)> {
    match (segment.end_cant_left, segment.end_cant_right) {
        (Some(left), Some(right)) => Ok((left, right)),
        _ => Err(AlignmentError::InvalidSegment {
            entity: segment.entity,
            detail: "transition cant segments require both end cant values",
        }),
    }
}

/// The shape fraction `f(ξ)` such that `D(ξ) = D1 + f(ξ)·(D2 - D1)`, for the
/// five types whose base formula is stated directly in that form.
fn shape_fraction(kind: &CantSegmentType, xi: f64) -> Option<f64> {
    match kind {
        CantSegmentType::LinearTransition => Some(xi),
        CantSegmentType::BlossCurve => Some((3.0 - 2.0 * xi) * xi * xi),
        CantSegmentType::CosineCurve => Some(0.5 * (1.0 - (std::f64::consts::PI * xi).cos())),
        CantSegmentType::SineCurve => Some(
            xi - (2.0 * std::f64::consts::PI).recip() * (2.0 * std::f64::consts::PI * xi).sin(),
        ),
        CantSegmentType::HelmertCurve => Some(if xi <= 0.5 {
            2.0 * xi * xi
        } else {
            1.0 - 2.0 * (1.0 - xi) * (1.0 - xi)
        }),
        _ => None,
    }
}

/// The section's cant `D = left - right` through the bend,
/// `ψ(ξ) = ψ1 + Δψ·ξ⁴·(35 − 84ξ + 70ξ² − 20ξ³)` with `ψ = arcsin(D / b)` and
/// `D = b·sin(ψ)`, and the rails `e ± D / 2` about a rotation point `e`
/// that stays put.
fn vienna_bend(
    segment: &CantSegment,
    xi: f64,
    b: f64,
    tolerance: SeamTolerance,
) -> AlignmentResult<CantAtStation> {
    let (end_left, end_right) = required_ends(segment)?;
    let start = CantAtStation {
        left: segment.start_cant_left,
        right: segment.start_cant_right,
    };
    let end = CantAtStation {
        left: end_left,
        right: end_right,
    };
    let ratio = |at: &CantAtStation| (at.left - at.right) / b;
    let (ratio_start, ratio_end) = (ratio(&start), ratio(&end));
    if !(-1.0..=1.0).contains(&ratio_start) || !(-1.0..=1.0).contains(&ratio_end) {
        return Err(AlignmentError::InvalidSegment {
            entity: segment.entity,
            detail: "VIENNESEBEND cant exceeds the rail head distance (|D| > b)",
        });
    }
    // The ends are authored; they need no law, whatever the pivot does.
    if xi == 0.0 {
        return Ok(start);
    }
    if xi == 1.0 {
        return Ok(end);
    }
    let pivot = 0.5 * (start.left + start.right);
    if !tolerance.same_length(pivot, 0.5 * (end.left + end.right)) {
        return Err(AlignmentError::Unsupported {
            entity: segment.entity,
            type_name: "VIENNESEBEND".to_owned(),
            detail: MOVING_VIENNESE_PIVOT,
        });
    }
    let psi_start = ratio_start.asin();
    let delta_psi = ratio_end.asin() - psi_start;
    let blend = xi.powi(4) * (35.0 - 84.0 * xi + 70.0 * xi * xi - 20.0 * xi * xi * xi);
    let half = 0.5 * b * (psi_start + delta_psi * blend).sin();
    Ok(CantAtStation {
        left: pivot + half,
        right: pivot - half,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ifc_model::EntityId;

    fn segment(
        predefined_type: CantSegmentType,
        start_left: f64,
        end_left: Option<f64>,
        start_right: f64,
        end_right: Option<f64>,
    ) -> CantSegment {
        CantSegment {
            entity: EntityId(1),
            start_dist_along: 0.0,
            horizontal_length: 100.0,
            start_cant_left: start_left,
            end_cant_left: end_left,
            start_cant_right: start_right,
            end_cant_right: end_right,
            predefined_type,
        }
    }

    #[test]
    fn constant_cant_holds_its_value_across_the_whole_segment() {
        let s = segment(CantSegmentType::ConstantCant, 0.15, None, -0.15, None);
        for xi in [0.0, 0.25, 0.5, 1.0] {
            let at = cant_at(&s, xi, None).expect("constant cant");
            assert_eq!(
                at,
                CantAtStation {
                    left: 0.15,
                    right: -0.15
                }
            );
        }
    }

    #[test]
    fn constant_cant_refuses_a_declared_end_that_disagrees_with_the_start() {
        let s = segment(CantSegmentType::ConstantCant, 0.15, Some(0.20), -0.15, None);
        assert!(cant_at(&s, 0.5, None).is_err());
    }

    #[test]
    fn linear_transition_interpolates_exactly() {
        let s = segment(
            CantSegmentType::LinearTransition,
            0.0,
            Some(0.20),
            0.0,
            Some(-0.20),
        );
        let at = cant_at(&s, 0.25, None).expect("linear cant");
        assert!((at.left - 0.05).abs() < 1e-12);
        assert!((at.right - (-0.05)).abs() < 1e-12);
    }

    #[test]
    fn bloss_curve_matches_the_published_base_formula_at_the_quarter_point() {
        let s = segment(CantSegmentType::BlossCurve, 0.0, Some(1.0), 0.0, Some(1.0));
        // f(0.25) = (3 - 0.5) * 0.0625 = 0.15625
        let at = cant_at(&s, 0.25, None).expect("bloss cant");
        assert!((at.left - 0.15625).abs() < 1e-12);
    }

    #[test]
    fn bloss_curve_is_symmetric_boundary_exact() {
        let s = segment(CantSegmentType::BlossCurve, 0.0, Some(1.0), 0.0, Some(1.0));
        assert!((cant_at(&s, 0.0, None).unwrap().left - 0.0).abs() < 1e-12);
        assert!((cant_at(&s, 1.0, None).unwrap().left - 1.0).abs() < 1e-12);
    }

    #[test]
    fn cosine_curve_matches_the_published_base_formula_at_the_midpoint() {
        let s = segment(CantSegmentType::CosineCurve, 0.0, Some(1.0), 0.0, Some(1.0));
        // f(0.5) = 0.5 * (1 - cos(pi/2)) = 0.5
        let at = cant_at(&s, 0.5, None).expect("cosine cant");
        assert!((at.left - 0.5).abs() < 1e-9);
    }

    #[test]
    fn sine_curve_starts_and_ends_exactly_on_the_endpoints() {
        let s = segment(CantSegmentType::SineCurve, 0.0, Some(1.0), 0.0, Some(1.0));
        assert!((cant_at(&s, 0.0, None).unwrap().left - 0.0).abs() < 1e-12);
        assert!((cant_at(&s, 1.0, None).unwrap().left - 1.0).abs() < 1e-9);
    }

    #[test]
    fn helmert_curve_is_continuous_across_its_own_midpoint_seam() {
        let s = segment(
            CantSegmentType::HelmertCurve,
            0.0,
            Some(1.0),
            0.0,
            Some(1.0),
        );
        let just_below = cant_at(&s, 0.5 - 1e-9, None).unwrap().left;
        let at_mid = cant_at(&s, 0.5, None).unwrap().left;
        let just_above = cant_at(&s, 0.5 + 1e-9, None).unwrap().left;
        assert!((just_below - at_mid).abs() < 1e-6);
        assert!((just_above - at_mid).abs() < 1e-6);
    }

    #[test]
    fn transitions_refuse_a_missing_end_value() {
        let s = segment(CantSegmentType::LinearTransition, 0.0, None, 0.0, None);
        assert!(cant_at(&s, 0.5, None).is_err());
    }

    #[test]
    fn vienna_bend_reproduces_the_endpoints_exactly() {
        let s = segment(
            CantSegmentType::VienneseBend,
            0.0,
            Some(0.15),
            0.0,
            Some(-0.15),
        );
        let start = cant_at(&s, 0.0, Some(1.5)).expect("vienna bend start");
        let end = cant_at(&s, 1.0, Some(1.5)).expect("vienna bend end");
        assert!((start.left - 0.0).abs() < 1e-9);
        assert!((end.left - 0.15).abs() < 1e-9);
        assert!((start.right - 0.0).abs() < 1e-9);
        assert!((end.right - (-0.15)).abs() < 1e-9);
    }

    /// The issue's example (#312): a rotation about the centreline from
    /// `D = 0` to `D = 0.15` over `b = 1.5`. At `xi = 0.5` the blend is
    /// exactly `1/2`, so the standard's `D = b sin(psi2 / 2)`, not the
    /// per-rail `2 b sin(arcsin(0.05) / 2)`.
    #[test]
    fn vienna_bend_blends_the_sections_cant_angle_not_each_rail() {
        let s = segment(
            CantSegmentType::VienneseBend,
            0.0,
            Some(0.075),
            0.0,
            Some(-0.075),
        );
        let at = cant_at(&s, 0.5, Some(1.5)).expect("vienna bend");
        let cant = at.left - at.right;
        let standard = 1.5 * (0.5 * 0.1_f64.asin()).sin();
        assert!((cant - standard).abs() < 1e-15, "{cant} != {standard}");
        assert!((cant - 0.075_094_2).abs() < 5e-8, "{cant}");
        let per_rail = 2.0 * 1.5 * (0.5 * 0.05_f64.asin()).sin();
        assert!((per_rail - 0.075_023_5).abs() < 5e-8, "{per_rail}");
        assert!((cant - per_rail).abs() > 7e-5, "the per-rail reading");
        // The rotation point stays on the centreline.
        assert!((at.left + at.right).abs() < 1e-15);
    }

    /// A rotation point raised 50 mm and held: rails `e +- D / 2`, with `D`
    /// from the angle law.
    #[test]
    fn vienna_bend_places_the_rails_about_a_pivot_that_stays_put() {
        let s = segment(
            CantSegmentType::VienneseBend,
            0.05,
            Some(0.125),
            0.05,
            Some(-0.025),
        );
        let psi2 = (0.15_f64 / 1.5).asin();
        for xi in [0.1_f64, 0.25, 0.5, 0.75, 0.9] {
            let blend = xi.powi(4) * (35.0 - 84.0 * xi + 70.0 * xi * xi - 20.0 * xi.powi(3));
            let d = 1.5 * (psi2 * blend).sin();
            let at = cant_at(&s, xi, Some(1.5)).expect("vienna bend");
            assert!((at.left - (0.05 + d / 2.0)).abs() < 1e-15, "left at {xi}");
            assert!((at.right - (0.05 - d / 2.0)).abs() < 1e-15, "right at {xi}");
        }
    }

    /// Rotation about the low rail moves the pivot: refused inside the
    /// bend, with the banked lowering's reason; the authored ends stand.
    #[test]
    fn vienna_bend_refuses_a_moving_pivot_inside_the_segment() {
        let s = segment(
            CantSegmentType::VienneseBend,
            0.0,
            Some(0.15),
            0.0,
            Some(0.0),
        );
        for xi in [1e-9_f64, 0.5, 1.0 - 1e-9] {
            assert_eq!(
                cant_at(&s, xi, Some(1.5)),
                Err(AlignmentError::Unsupported {
                    entity: EntityId(1),
                    type_name: "VIENNESEBEND".to_owned(),
                    detail: MOVING_VIENNESE_PIVOT,
                }),
                "at {xi}"
            );
        }
        let start = cant_at(&s, 0.0, Some(1.5)).expect("authored start");
        let end = cant_at(&s, 1.0, Some(1.5)).expect("authored end");
        assert_eq!(
            (start.left, start.right, end.left, end.right),
            (0.0, 0.0, 0.15, 0.0)
        );
    }

    /// Rotation points that differ within the model's precision stay put,
    /// as the banked lowering compares them; strictly they move.
    #[test]
    fn vienna_bend_compares_pivots_at_the_given_tolerance() {
        let s = segment(
            CantSegmentType::VienneseBend,
            0.0,
            Some(0.075_002),
            0.0,
            Some(-0.075),
        );
        assert!(matches!(
            cant_at(&s, 0.5, Some(1.5)),
            Err(AlignmentError::Unsupported { .. })
        ));
        let precise = SeamTolerance::from_precision(1e-5).expect("precision");
        let at = cant_within(&s, 0.5, Some(1.5), precise).expect("pivot within precision");
        assert!((at.left + at.right).abs() < 1e-15, "the start pivot holds");
    }

    #[test]
    fn vienna_bend_refuses_without_a_rail_head_distance() {
        let s = segment(
            CantSegmentType::VienneseBend,
            0.0,
            Some(0.15),
            0.0,
            Some(-0.15),
        );
        assert!(cant_at(&s, 0.5, None).is_err());
    }

    #[test]
    fn vienna_bend_refuses_cant_exceeding_the_rail_head_distance() {
        let s = segment(
            CantSegmentType::VienneseBend,
            0.0,
            Some(2.0),
            0.0,
            Some(-0.15),
        );
        assert!(cant_at(&s, 0.5, Some(1.5)).is_err());
    }

    #[test]
    fn user_defined_predefined_type_is_a_typed_refusal_not_a_guess() {
        let s = segment(CantSegmentType::UserDefined, 0.0, Some(1.0), 0.0, Some(1.0));
        assert!(matches!(
            cant_at(&s, 0.5, None),
            Err(AlignmentError::Unsupported { .. })
        ));
    }
}
