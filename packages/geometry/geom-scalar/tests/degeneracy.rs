//! Escalation-rate gates across degeneracy tiers.
//!
//! "Does robustness collapse on bad data" is answered by two numbers per tier:
//! how often the filter defers to exact arithmetic, and whether the answers
//! stay correct when it does. Both are asserted here so a regression in either
//! the bound or the exact path shows up as a failing test rather than a slow
//! program.

use geom_scalar::scene::{orient2_scene, orient3_scene, DegeneracyRate};
use geom_scalar::{orient2d, orient2d_filter, orient3d, orient3d_filter};

const SAMPLES: usize = 100_000;

/// Fraction of cases the fast filter could not settle.
fn escalation_rate_2d(rate: DegeneracyRate) -> f64 {
    let scene = orient2_scene(SAMPLES, rate, 0x51ED_2701);
    let escalated = scene
        .iter()
        .filter(|case| !orient2d_filter(case[0], case[1], case[2]).is_certain())
        .count();
    escalated as f64 / SAMPLES as f64
}

fn escalation_rate_3d(rate: DegeneracyRate) -> f64 {
    let scene = orient3_scene(SAMPLES, rate, 0x51ED_2701);
    let escalated = scene
        .iter()
        .filter(|case| !orient3d_filter(case[0], case[1], case[2], case[3]).is_certain())
        .count();
    escalated as f64 / SAMPLES as f64
}

/// Escalation must track the degeneracy rate, and clean data must be almost
/// free.
///
/// The upper bounds are the load-bearing assertions: they are what would fail
/// if a future change loosened a filter bound and pushed ordinary geometry
/// onto the expensive path.
#[test]
fn escalation_tracks_degeneracy_and_clean_data_stays_cheap() {
    let clean = escalation_rate_2d(DegeneracyRate::None);
    assert!(
        clean < 0.001,
        "clean data escalated {:.4}% of the time; the filter is not doing its job",
        clean * 100.0
    );

    for rate in DegeneracyRate::ALL {
        let measured = escalation_rate_2d(rate);
        // Every degenerate case MUST escalate: an exactly collinear triple has
        // determinant zero, which no error bound can exclude.
        assert!(
            measured >= rate.fraction() * 0.99,
            "{}: escalated {:.4} but {:.4} of inputs are exactly degenerate",
            rate.label(),
            measured,
            rate.fraction()
        );
        // And it must not escalate much MORE than the degenerate fraction,
        // which would mean the bound is too loose.
        assert!(
            measured <= rate.fraction() + 0.005,
            "{}: escalated {:.4}, far above the {:.4} degenerate fraction",
            rate.label(),
            measured,
            rate.fraction()
        );
    }
}

/// The same property in 3D, where the determinant is degree 3 and the bound
/// is correspondingly looser.
#[test]
fn three_dimensional_escalation_also_tracks_degeneracy() {
    let clean = escalation_rate_3d(DegeneracyRate::None);
    assert!(
        clean < 0.001,
        "clean 3D data escalated {:.4}%",
        clean * 100.0
    );
    for rate in DegeneracyRate::ALL {
        let measured = escalation_rate_3d(rate);
        assert!(
            measured >= rate.fraction() * 0.99,
            "{}: 3D escalated {measured:.4}, below the degenerate fraction",
            rate.label()
        );
        assert!(
            measured <= rate.fraction() + 0.005,
            "{}: 3D escalated {measured:.4}, far above the degenerate fraction",
            rate.label()
        );
    }
}

/// Correctness must not degrade as degeneracy rises.
///
/// This is the actual "does robustness collapse" question. Every degenerate
/// case in the scene is exactly collinear or coplanar by construction, so the
/// expected sign is known without an oracle: it must be exactly zero.
#[test]
fn degenerate_cases_are_answered_exactly_at_every_tier() {
    use geom_kernel::Sign;

    for rate in DegeneracyRate::ALL {
        let scene = orient2_scene(20_000, rate, 0xC0FF_EE01);
        let mut degenerate_seen = 0usize;
        for (index, case) in scene.iter().enumerate() {
            let sign = orient2d(case[0], case[1], case[2])
                .sign()
                .expect("orient2d always certifies");
            if rate.is_degenerate(index) {
                assert_eq!(
                    sign,
                    Sign::Zero,
                    "{}: a constructed-collinear case at {index} was not exactly zero",
                    rate.label()
                );
                degenerate_seen += 1;
            }
        }
        if rate != DegeneracyRate::None {
            assert!(
                degenerate_seen > 0,
                "{}: the scene contained no degenerate cases",
                rate.label()
            );
        }
    }

    for rate in DegeneracyRate::ALL {
        let scene = orient3_scene(20_000, rate, 0xC0FF_EE01);
        for (index, case) in scene.iter().enumerate() {
            let sign = orient3d(case[0], case[1], case[2], case[3])
                .sign()
                .expect("orient3d always certifies");
            if rate.is_degenerate(index) {
                assert_eq!(
                    sign,
                    Sign::Zero,
                    "{}: a constructed-coplanar case at {index} was not exactly zero",
                    rate.label()
                );
            }
        }
    }
}
