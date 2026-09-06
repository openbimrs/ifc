//! Transcriptions of the schema's own normative EXPRESS functions.
//!
//! # Why these live in one module
//!
//! Several WHERE rules delegate to a named EXPRESS FUNCTION rather than
//! stating a predicate inline. Those functions are normative text: the
//! rule is only enforced correctly if the function is transcribed
//! faithfully, including its edge cases. Keeping them together, named
//! after their schema counterparts, makes them checkable against the
//! specification line by line.
//!
//! # Indices are one-based
//!
//! EXPRESS lists are indexed from 1 and the B-spline arrays from 0. The
//! transcriptions below keep the schema's own bounds in the comments and
//! translate to Rust slices at the point of access, so a reader can
//! compare each line with the specification without re-deriving offsets.

/// `IfcConstraintsParamBSpline`: is this a valid B-spline parametrisation?
///
/// `up_knots` and `up_cp` are the schema's upper indices, which are
/// derived: `UpperIndexOnKnots := SIZEOF(Knots)` and
/// `UpperIndexOnControlPoints := SIZEOF(ControlPointsList) - 1`. They are
/// passed in rather than recomputed so each caller states which list it
/// measured.
pub fn constraints_param_bspline(
    degree: i64,
    up_knots: i64,
    up_cp: i64,
    knot_mult: &[i64],
    knots: &[f64],
) -> bool {
    // The function indexes KnotMult[1..UpKnots]; a shorter list means the
    // file cannot satisfy it.
    if knot_mult.is_empty() || (up_knots as usize) > knot_mult.len() {
        return false;
    }

    // Sum of knot multiplicities over 1..UpKnots.
    let sum: i64 = knot_mult.iter().take(up_knots as usize).sum();

    // Limits holding for all B-spline parametrisations.
    if degree < 1 || up_knots < 2 || up_cp < degree || sum != degree + up_cp + 2 {
        return false;
    }

    // First multiplicity: 1 <= K <= Degree + 1.
    let k = knot_mult[0];
    if k < 1 || k > degree + 1 {
        return false;
    }

    // Knots must be strictly increasing, and interior multiplicities are
    // capped at Degree while the final one may reach Degree + 1.
    for i in 2..=up_knots {
        let idx = (i - 1) as usize;
        if idx >= knot_mult.len() || idx >= knots.len() {
            return false;
        }
        if knot_mult[idx] < 1 || knots[idx] <= knots[idx - 1] {
            return false;
        }
        let k = knot_mult[idx];
        if i < up_knots && k > degree {
            return false;
        }
        if i == up_knots && k > degree + 1 {
            return false;
        }
    }
    true
}

/// `IfcConsecutiveSegments`: each segment must end where the next begins.
///
/// A segment is a list of point indices, so the join condition compares the
/// last index of one against the first index of the next. An empty segment
/// has no endpoint to compare and is reported as non-consecutive rather
/// than skipped, because it cannot participate in a continuous path.
pub fn consecutive_segments(segments: &[Vec<i64>]) -> bool {
    for pair in segments.windows(2) {
        let (a, b) = (&pair[0], &pair[1]);
        let (Some(end), Some(start)) = (a.last(), b.first()) else {
            return false;
        };
        if end != start {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A clamped cubic with four control points: the textbook case.
    ///
    /// Degree 3, UpCp 3, knots [0,1] with multiplicities [4,4]: the sum is
    /// 8 = Degree + UpCp + 2, and the final multiplicity may reach
    /// Degree + 1.
    #[test]
    fn a_clamped_cubic_is_a_valid_parametrisation() {
        assert!(constraints_param_bspline(3, 2, 3, &[4, 4], &[0.0, 1.0]));
    }

    #[test]
    fn the_multiplicity_sum_must_equal_degree_plus_upcp_plus_two() {
        // Same shape, one multiplicity short: 7 != 8.
        assert!(!constraints_param_bspline(3, 2, 3, &[4, 3], &[0.0, 1.0]));
    }

    #[test]
    fn knots_must_strictly_increase() {
        // Sum is right, but the knot vector repeats a value.
        assert!(!constraints_param_bspline(3, 2, 3, &[4, 4], &[1.0, 1.0]));
    }

    #[test]
    fn an_interior_multiplicity_may_not_exceed_the_degree() {
        // Three knots, interior multiplicity 4 > Degree 3. Sum is
        // 3 + 4 + 3 = 10 = Degree + UpCp + 2 with UpCp = 5.
        assert!(!constraints_param_bspline(
            3,
            3,
            5,
            &[3, 4, 3],
            &[0.0, 0.5, 1.0]
        ));
    }

    /// The FIRST multiplicity is bounded by Degree + 1, like the last.
    ///
    /// Sum, ordering and interior caps can all hold while the opening
    /// multiplicity overruns, so this needs its own case.
    #[test]
    fn the_first_multiplicity_may_not_exceed_degree_plus_one() {
        // Degree 2, knots [0,1], multiplicities [4,3]: sum 7 = 2 + 3 + 2,
        // knots increase, the final multiplicity is within Degree + 1,
        // but the first is 4 > 3.
        assert!(!constraints_param_bspline(2, 2, 3, &[4, 3], &[0.0, 1.0]));
    }

    #[test]
    fn degree_zero_and_a_single_knot_are_rejected() {
        assert!(!constraints_param_bspline(0, 2, 3, &[4, 4], &[0.0, 1.0]));
        assert!(!constraints_param_bspline(3, 1, 3, &[4], &[0.0]));
    }

    #[test]
    fn fewer_control_points_than_the_degree_is_rejected() {
        assert!(!constraints_param_bspline(3, 2, 2, &[4, 3], &[0.0, 1.0]));
    }

    #[test]
    fn segments_join_end_to_start() {
        assert!(consecutive_segments(&[vec![1, 2, 3], vec![3, 4]]));
        assert!(!consecutive_segments(&[vec![1, 2, 3], vec![9, 4]]));
        // A single segment has no join to check.
        assert!(consecutive_segments(&[vec![1, 2]]));
    }
}
