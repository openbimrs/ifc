//! Gates for the static filter.
//!
//! The safety property is one-directional: whenever the static filter commits
//! to a sign, that sign must equal the certified one. It is allowed to decline
//! (that is the price of a looser bound); it is never allowed to be wrong.

use geom_core::{Point2, Point3};
use geom_scalar::{orient2d, orient3d, StaticFilter};

#[test]
fn an_invalid_range_is_refused_rather_than_silently_accepted() {
    assert!(StaticFilter::new(0.0).is_none());
    assert!(StaticFilter::new(-1.0).is_none());
    assert!(StaticFilter::new(f64::NAN).is_none());
    assert!(StaticFilter::new(f64::INFINITY).is_none());
    // A bound whose derived error bound overflows must be refused too, rather
    // than yielding a filter that certifies nothing.
    assert!(StaticFilter::new(f64::MAX).is_none());
}

#[test]
fn a_point_outside_the_declared_range_is_declined() {
    let filter = StaticFilter::new(10.0).expect("valid");
    let inside = Point2::new(1.0, 1.0);
    let outside = Point2::new(1e6, 0.0);
    assert!(filter.orient2d(inside, outside, inside).is_none());
}

/// Whenever the static filter answers, it must agree with the exact predicate.
#[test]
fn a_static_answer_never_contradicts_the_exact_predicate() {
    let bound = 1_000.0;
    let filter = StaticFilter::new(bound).expect("valid");
    let mut state = 0x2545_F491_4F6C_DD1Du64;
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        // Integer coordinates well inside the declared range.
        (((state >> 33) as i64) % 2_000 - 1_000) as f64
    };

    let mut answered = 0usize;
    for _ in 0..50_000 {
        let a = Point2::new(next(), next());
        let b = Point2::new(next(), next());
        let c = Point2::new(next(), next());
        if let Some(fast) = filter.orient2d(a, b, c) {
            let exact = orient2d(a, b, c).sign().expect("certified");
            assert_eq!(fast, exact, "static filter disagreed on {a:?} {b:?} {c:?}");
            answered += 1;
        }
    }
    // A filter that always declined would pass the assertion above vacuously.
    assert!(
        answered > 40_000,
        "static filter answered only {answered}/50000; it is not earning its cost"
    );
}

/// The same one-directional safety property in 3D, including a deliberate
/// mix of degenerate inputs where the static bound must decline rather than
/// commit to a wrong sign.
#[test]
fn the_three_dimensional_static_answer_is_also_safe() {
    let filter = StaticFilter::new(1_000.0).expect("valid");
    let mut state = 0x9E37_79B9_7F4A_7C15u64;
    let mut next = |m: i64| {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        (((state >> 33) as i64) % (2 * m) - m) as f64
    };

    let mut answered = 0usize;
    let mut declined_on_degenerate = 0usize;
    for i in 0..50_000 {
        let a = Point3::new(next(500), next(500), next(500));
        let b = Point3::new(next(500), next(500), next(500));
        let c = Point3::new(next(500), next(500), next(500));
        // Every other case is forced exactly onto the plane through a, b, c,
        // where the true determinant is zero and no bound may certify a sign.
        let d = if i % 2 == 0 {
            Point3::new(next(500), next(500), next(500))
        } else {
            Point3::new(
                a.x + (b.x - a.x) + (c.x - a.x),
                a.y + (b.y - a.y) + (c.y - a.y),
                a.z + (b.z - a.z) + (c.z - a.z),
            )
        };
        match filter.orient3d(a, b, c, d) {
            Some(fast) => {
                let exact = orient3d(a, b, c, d).sign().expect("certified");
                assert_eq!(fast, exact, "static filter disagreed in 3D");
                answered += 1;
            }
            None if i % 2 == 1 => declined_on_degenerate += 1,
            None => {}
        }
    }
    assert!(
        answered > 20_000,
        "3D static filter answered only {answered}"
    );
    assert!(
        declined_on_degenerate > 20_000,
        "coplanar inputs must be declined, not certified: {declined_on_degenerate}"
    );
}
