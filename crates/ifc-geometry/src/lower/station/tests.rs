//! Seams read from stored curve data.

use axiolid_core::{Point2, Point3};
use axiolid_curve::linear::{Polyline2, Polyline3};
use axiolid_curve::{Curve2, Curve3, ElevationLaw};

use super::seams::{stated_length, tangent_seams};
use crate::lower::session::AtomicCurve;

fn polyline3(points: &[[f64; 3]], closed: bool) -> AtomicCurve {
    AtomicCurve::Three(Curve3::Polyline(Polyline3 {
        points: points
            .iter()
            .map(|p| Point3::new(p[0], p[1], p[2]))
            .collect(),
        closed,
    }))
}

/// A corner is a seam at its distance; a collinear vertex is not; a closed
/// polyline's closing corner counts, its start does not.
#[test]
fn polyline_corners_are_seams_and_collinear_vertices_are_not() {
    let open = polyline3(
        &[
            [0.0, 0.0, 0.0],
            [3.0, 0.0, 0.0],
            [5.0, 0.0, 0.0],
            [5.0, 4.0, 0.0],
        ],
        false,
    );
    assert_eq!(tangent_seams(&open), Ok(vec![5.0]));
    assert_eq!(stated_length(&open), Some(9.0));

    let square = AtomicCurve::Two(Curve2::Polyline(Polyline2 {
        points: [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]
            .iter()
            .map(|p| Point2::new(p[0], p[1]))
            .collect(),
        closed: true,
    }));
    assert_eq!(tangent_seams(&square), Ok(vec![1.0, 2.0, 3.0]));
    assert_eq!(stated_length(&square), Some(4.0));

    // A reversal is a seam too.
    let back = polyline3(&[[0.0, 0.0, 0.0], [2.0, 0.0, 0.0], [1.0, 0.0, 0.0]], false);
    assert_eq!(tangent_seams(&back), Ok(vec![2.0]));
}

/// An elevation break is a seam only where the grade jumps.
#[test]
fn elevation_breaks_are_seams_where_the_grade_jumps() {
    let elevated = |laws: Vec<ElevationLaw>, breaks: Vec<f64>| {
        let plan = Curve2::Polyline(Polyline2 {
            points: vec![Point2::new(0.0, 0.0), Point2::new(100.0, 0.0)],
            closed: false,
        });
        AtomicCurve::Three(Curve3::Elevated(axiolid_curve::Elevated3::new(
            plan,
            ElevationLaw::Piecewise { breaks, laws },
        )))
    };
    // 2 % for 30 m, then a parabola leaving at 2 %: tangent.
    let smooth = elevated(
        vec![
            ElevationLaw::constant_grade(0.0, 0.02),
            ElevationLaw::Polynomial {
                coefficients: vec![0.6, 0.02, 0.001],
            },
        ],
        vec![30.0],
    );
    assert_eq!(tangent_seams(&smooth), Ok(vec![]));
    assert_eq!(stated_length(&smooth), Some(100.0));
    // 2 % then -1 %: a kink at 30 m.
    let kink = elevated(
        vec![
            ElevationLaw::constant_grade(0.0, 0.02),
            ElevationLaw::constant_grade(0.6, -0.01),
        ],
        vec![30.0],
    );
    assert_eq!(tangent_seams(&kink), Ok(vec![30.0]));
}
