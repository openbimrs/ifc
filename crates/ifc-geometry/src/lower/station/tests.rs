//! Seams read from stored curve data.

use axiolid_core::{Point2, Point3};
use axiolid_curve::linear::{Polyline2, Polyline3};
use axiolid_curve::{Curve2, Curve3, ElevationLaw};

use axiolid_evaluate::station::{exact_station_seams2, exact_station_seams3, MITRE_TOLERANCE};

use super::seams::{self, stated_length, tangent_seams, Seam};
use crate::lower::session::AtomicCurve;

/// The seam distances, or the refusal.
fn distances(curve: &AtomicCurve) -> Result<Vec<f64>, &'static str> {
    tangent_seams(curve).map(|seams| seams.iter().map(|seam| seam.distance).collect())
}

/// Axiolid's exact seams where the frame may jump (#263), for comparison.
fn axiolid_seams(curve: &AtomicCurve) -> Vec<f64> {
    match curve {
        AtomicCurve::Two(curve) => exact_station_seams2(curve),
        AtomicCurve::Three(curve) => exact_station_seams3(curve),
    }
    .expect("exact seams")
    .into_iter()
    .filter(|seam| !seam.smooth)
    .map(|seam| seam.distance)
    .collect()
}

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
    assert_eq!(distances(&open), Ok(vec![5.0]));
    assert_eq!(stated_length(&open), Some(9.0));

    let square = AtomicCurve::Two(Curve2::Polyline(Polyline2 {
        points: [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]
            .iter()
            .map(|p| Point2::new(p[0], p[1]))
            .collect(),
        closed: true,
    }));
    assert_eq!(distances(&square), Ok(vec![1.0, 2.0, 3.0]));
    assert_eq!(stated_length(&square), Some(4.0));

    // A reversal is a seam too, and one a run cannot be mitred across.
    let back = polyline3(&[[0.0, 0.0, 0.0], [2.0, 0.0, 0.0], [1.0, 0.0, 0.0]], false);
    assert_eq!(
        tangent_seams(&back),
        Ok(vec![Seam {
            distance: 2.0,
            reverses: true
        }])
    );
    // A right angle is mitred.
    assert!(tangent_seams(&open)
        .expect("seams")
        .iter()
        .all(|s| !s.reverses));
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
    assert_eq!(distances(&smooth), Ok(vec![]));
    assert_eq!(stated_length(&smooth), Some(100.0));
    // 2 % then -1 %: a kink at 30 m.
    let kink = elevated(
        vec![
            ElevationLaw::constant_grade(0.0, 0.02),
            ElevationLaw::constant_grade(0.6, -0.01),
        ],
        vec![30.0],
    );
    assert_eq!(distances(&kink), Ok(vec![30.0]));
    assert_eq!(axiolid_seams(&kink), vec![30.0]);
}

/// The seams read here are Axiolid's (`exact_station_seams`, #263) at the
/// same distances, so a station snapped to one is ON it for the kernel:
/// every turning vertex of a polyline whose edges have irrational lengths,
/// to the bit, Axiolid also listing the collinear vertex this skips.
#[test]
fn seams_are_axiolids_exact_seams() {
    let points = [
        [0.0, 0.0, 0.0],
        [3.1, 0.7, 0.2],
        [5.3, 4.1, 0.2],
        [9.9, 4.4, 1.3],
        [14.5, 4.7, 2.4],
        [12.0, 9.0, 2.4],
    ];
    let curve = polyline3(&points, false);
    let ours = distances(&curve).expect("seams");
    let theirs = axiolid_seams(&curve);
    assert_eq!(theirs.len(), ours.len() + 1, "{theirs:?} vs {ours:?}");
    let collinear = theirs[2];
    let turning: Vec<f64> = theirs.into_iter().filter(|d| *d != collinear).collect();
    assert_eq!(ours, turning, "bit for bit");

    let plan = AtomicCurve::Two(Curve2::Polyline(Polyline2 {
        points: points.iter().map(|p| Point2::new(p[0], p[1])).collect(),
        closed: true,
    }));
    let ours = distances(&plan).expect("seams");
    let theirs = axiolid_seams(&plan);
    for seam in &ours {
        assert!(theirs.contains(seam), "{seam} not in {theirs:?}");
    }
}

/// The reversal threshold is Axiolid's mitre limit, which this crate cannot
/// link (ADR 0004).
#[test]
fn the_reversal_threshold_is_axiolids_mitre_tolerance() {
    assert_eq!(seams::MITRE_TOLERANCE, MITRE_TOLERANCE);
}
