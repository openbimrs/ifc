//! `IfcBaseAxis`: deriving an orthonormal frame from optional axis hints.
//!
//! A transformation operator names its axes `Axis1`/`Axis2`/`Axis3` instead of
//! a placement's `RefDirection`/`Axis`, all of them optional, and the schema
//! function `IfcBaseAxis` says how to turn whichever subset the file supplied
//! into a full frame. The 2D and 3D cases follow different rules, so they live
//! here rather than being open-coded per operator subtype.
//!
//! # What `Axis2` actually contributes
//!
//! Nothing about direction: the derived Y is forced orthogonal to X (and, in
//! 3D, to Z) whatever `Axis2` says. Its **sign** is all that survives, and
//! that is precisely how a mirroring operator is expressed. Ignoring it turns
//! a mirrored component into an unmirrored one, which reads as correct
//! geometry facing the wrong way -- the worst kind of wrong.

use crate::transform::Transform;

/// The 3D case of `IfcBaseAxis`.
///
/// `Axis3` is the frame's Z and `Axis1` its approximate X, which is the
/// reverse naming of a placement's `Axis`/`RefDirection` but the identical
/// construction, so the Gram-Schmidt projection is delegated to
/// [`Transform::from_axes`] rather than repeated.
///
/// Returns `None` when the axes are degenerate: zero-length, or `Axis1`
/// parallel to `Axis3` so that no X survives the projection.
pub fn base_axes_3d(
    origin: [f64; 3],
    axis1: Option<[f64; 3]>,
    axis2: Option<[f64; 3]>,
    axis3: Option<[f64; 3]>,
) -> Option<Transform> {
    let frame = Transform::from_axes(origin, axis3, axis1)?;
    Some(match axis2 {
        Some(a2) if dot(a2, frame.basis[1]) < 0.0 => Transform {
            basis: [frame.basis[0], negate(frame.basis[1]), frame.basis[2]],
            origin,
        },
        _ => frame,
    })
}

/// The 2D case of `IfcBaseAxis`.
///
/// Returned as a 3D transform lying in the z=0 plane so consumers never carry
/// two transform types. Y is the orthogonal complement of X (X turned a
/// quarter turn counter-clockwise), flipped when `Axis2` opposes it.
///
/// When only `Axis2` is given, X is derived *from it* by the reverse quarter
/// turn, which is why this cannot simply call [`Transform::from_axes`] with a
/// default X: there is no 3D analogue of that rule.
pub fn base_axes_2d(
    origin: [f64; 3],
    axis1: Option<[f64; 3]>,
    axis2: Option<[f64; 3]>,
) -> Option<Transform> {
    let x = match (axis1, axis2) {
        (Some(a1), _) => [a1[0], a1[1]],
        // Axis2 alone: X is Y turned a quarter turn clockwise.
        (None, Some(a2)) => [a2[1], -a2[0]],
        (None, None) => [1.0, 0.0],
    };
    let x = normalize_2d(x)?;
    let mut y = [-x[1], x[0]];
    // Only meaningful when Axis1 fixed X. If Y came from Axis2 it already
    // agrees with it by construction.
    if let (Some(_), Some(a2)) = (axis1, axis2) {
        if a2[0] * y[0] + a2[1] * y[1] < 0.0 {
            y = [-y[0], -y[1]];
        }
    }
    Some(Transform {
        basis: [[x[0], x[1], 0.0], [y[0], y[1], 0.0], [0.0, 0.0, 1.0]],
        origin,
    })
}

/// Normalize a 2D vector, or `None` when it is too short to have a direction.
fn normalize_2d(v: [f64; 2]) -> Option<[f64; 2]> {
    let len = (v[0] * v[0] + v[1] * v[1]).sqrt();
    if len < 1e-12 {
        return None;
    }
    Some([v[0] / len, v[1] / len])
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn negate(v: [f64; 3]) -> [f64; 3] {
    [-v[0], -v[1], -v[2]]
}

#[cfg(test)]
mod tests {
    use super::*;

    const O: [f64; 3] = [0.0, 0.0, 0.0];

    fn close(a: [f64; 3], b: [f64; 3]) -> bool {
        a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-9)
    }

    #[test]
    fn omitting_every_axis_gives_the_global_frame() {
        assert_eq!(
            base_axes_3d(O, None, None, None).unwrap(),
            Transform::identity()
        );
        assert_eq!(base_axes_2d(O, None, None).unwrap(), Transform::identity());
    }

    #[test]
    fn axis1_is_the_local_x_and_axis3_the_local_z() {
        let t = base_axes_3d(O, Some([0.0, 1.0, 0.0]), None, Some([1.0, 0.0, 0.0])).unwrap();
        assert!(close(t.basis[0], [0.0, 1.0, 0.0]), "X follows Axis1");
        assert!(close(t.basis[2], [1.0, 0.0, 0.0]), "Z follows Axis3");
    }

    /// The only information `Axis2` can still carry once Y is forced
    /// orthogonal is its sign, and that sign is the mirror flag.
    #[test]
    fn an_opposing_axis2_flips_handedness_rather_than_being_ignored() {
        let upright = base_axes_3d(O, Some([1.0, 0.0, 0.0]), Some([0.0, 1.0, 0.0]), None).unwrap();
        let mirrored =
            base_axes_3d(O, Some([1.0, 0.0, 0.0]), Some([0.0, -1.0, 0.0]), None).unwrap();
        assert!(close(upright.basis[1], [0.0, 1.0, 0.0]));
        assert!(close(mirrored.basis[1], [0.0, -1.0, 0.0]));
    }

    /// A non-perpendicular `Axis2` still only decides sign; Y stays orthogonal.
    #[test]
    fn axis2_never_shears_the_frame() {
        let t = base_axes_3d(O, Some([1.0, 0.0, 0.0]), Some([0.3, 0.9, 0.0]), None).unwrap();
        assert!(close(t.basis[1], [0.0, 1.0, 0.0]));
    }

    #[test]
    fn two_dimensional_y_is_x_turned_a_quarter_turn_counter_clockwise() {
        let t = base_axes_2d(O, Some([0.0, 1.0, 0.0]), None).unwrap();
        assert!(close(t.basis[0], [0.0, 1.0, 0.0]));
        assert!(close(t.basis[1], [-1.0, 0.0, 0.0]));
    }

    /// `IfcBaseAxis` derives X from `Axis2` when `Axis1` is absent; defaulting
    /// X to global X instead would silently rotate the frame.
    #[test]
    fn axis2_alone_determines_x_in_two_dimensions() {
        let t = base_axes_2d(O, None, Some([0.0, 1.0, 0.0])).unwrap();
        assert!(close(t.basis[0], [1.0, 0.0, 0.0]));
        assert!(close(t.basis[1], [0.0, 1.0, 0.0]));
    }

    #[test]
    fn unnormalized_axes_still_yield_a_unit_frame() {
        let t = base_axes_2d(O, Some([7.0, 0.0, 0.0]), None).unwrap();
        assert!(close(t.basis[0], [1.0, 0.0, 0.0]));
    }

    #[test]
    fn zero_length_axes_are_rejected_rather_than_producing_nan() {
        assert!(base_axes_2d(O, Some([0.0, 0.0, 0.0]), None).is_none());
        assert!(base_axes_3d(O, None, None, Some([0.0, 0.0, 0.0])).is_none());
    }

    /// Parallel X and Z leave nothing after the projection.
    #[test]
    fn axis1_parallel_to_axis3_is_degenerate() {
        assert!(base_axes_3d(O, Some([0.0, 0.0, 2.0]), None, Some([0.0, 0.0, 1.0])).is_none());
    }

    /// A 3D `Axis1` in a 2D operator contributes only its XY part; keeping z
    /// would tilt a frame that is required to lie in the z=0 plane.
    #[test]
    fn two_dimensional_axes_ignore_any_z_component() {
        let t = base_axes_2d(O, Some([1.0, 0.0, 5.0]), None).unwrap();
        assert_eq!(t.basis[2], [0.0, 0.0, 1.0]);
        assert!(close(t.basis[0], [1.0, 0.0, 0.0]));
    }
}
