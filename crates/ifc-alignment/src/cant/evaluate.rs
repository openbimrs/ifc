//! Exact closed-form cant variation `D(s)` for each defined segment type.
//!
//! Every type in `IfcAlignmentCantSegmentTypeEnum` (buildingSMART IFC4X3
//! §8.7.2.1 base formulas) states cant directly as a value `D(ξ)` of the
//! normalized position `ξ = s / L`, not as an integral of a curvature
//! function. That is the crucial difference from the horizontal/vertical
//! transition families: cant transition never requires Fresnel-type
//! integration, so every defined type here is exactly representable.
//!
//! `VIENNESEBEND` is the one type that routes through an angle: it defines
//! the bank angle `ψ(ξ)` by a closed-form quartic, and the spec's own
//! relation `ψ = arcsin(D / b)` inverts to `D = b · sin(ψ)` exactly -- `sin`
//! is an elementary function, not a transcendental integral, so this stays
//! exact too.

use crate::cant::segment::{CantSegment, CantSegmentType};
use crate::error::{AlignmentError, AlignmentResult};

/// Cant applied to each rail at one normalized position along a segment.
#[derive(Debug, Clone, Copy, PartialEq)]
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
pub fn cant_at(
    segment: &CantSegment,
    xi: f64,
    rail_head_distance: Option<f64>,
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
            let (end_left, end_right) = required_ends(segment)?;
            let left = vienna_bend(segment, segment.start_cant_left, end_left, xi, b)?;
            let right = vienna_bend(segment, segment.start_cant_right, end_right, xi, b)?;
            Ok(CantAtStation { left, right })
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

/// `ψ(ξ) = ψ1 + Δψ·ξ⁴·(35 − 84ξ + 70ξ² − 20ξ³)`, then `D = b·sin(ψ)`.
fn vienna_bend(
    segment: &CantSegment,
    start: f64,
    end: f64,
    xi: f64,
    b: f64,
) -> AlignmentResult<f64> {
    let ratio_start = start / b;
    let ratio_end = end / b;
    if !(-1.0..=1.0).contains(&ratio_start) || !(-1.0..=1.0).contains(&ratio_end) {
        return Err(AlignmentError::InvalidSegment {
            entity: segment.entity,
            detail: "VIENNESEBEND cant exceeds the rail head distance (|D| > b)",
        });
    }
    let psi_start = ratio_start.asin();
    let psi_end = ratio_end.asin();
    let delta_psi = psi_end - psi_start;
    let blend = xi.powi(4) * (35.0 - 84.0 * xi + 70.0 * xi * xi - 20.0 * xi * xi * xi);
    let psi = psi_start + delta_psi * blend;
    Ok(b * psi.sin())
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
