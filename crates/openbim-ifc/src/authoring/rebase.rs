//! The rigid transform of an `IfcAxis2Placement3D`, for re-basing a
//! placement whose parent is removed.
//!
//! An `IfcLocalPlacement` maps its local coordinates into its parent's by
//! its `IfcAxis2Placement3D`: the origin `Location`, local Z `Axis`
//! (default `(0,0,1)`), and local X from `RefDirection` (default
//! `(1,0,0)`) projected orthogonal to Z, as `IfcBuildAxes` derives them.
//! Re-basing a placement past a removed parent composes the parent's
//! transform with its own, so every point stays where it was.

/// A rigid transform: an origin and three orthonormal axes, in the
/// parent's coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Transform {
    pub(super) origin: [f64; 3],
    pub(super) x: [f64; 3],
    pub(super) y: [f64; 3],
    pub(super) z: [f64; 3],
}

impl Transform {
    /// The transform of an axis placement, `None` when the axes are
    /// degenerate (zero, or parallel).
    pub(super) fn from_axes(
        origin: [f64; 3],
        axis: Option<[f64; 3]>,
        ref_direction: Option<[f64; 3]>,
    ) -> Option<Self> {
        let z = normalize(axis.unwrap_or([0.0, 0.0, 1.0]))?;
        let r = ref_direction.unwrap_or([1.0, 0.0, 0.0]);
        let along = dot(r, z);
        let x = normalize([
            r[0] - along * z[0],
            r[1] - along * z[1],
            r[2] - along * z[2],
        ])?;
        let y = cross(z, x);
        Some(Self { origin, x, y, z })
    }

    /// `self` after `child`: the transform of a placement whose parent is
    /// `self` and whose own transform is `child`.
    pub(super) fn then(&self, child: &Self) -> Self {
        let rotate = |v: [f64; 3]| {
            [
                v[0] * self.x[0] + v[1] * self.y[0] + v[2] * self.z[0],
                v[0] * self.x[1] + v[1] * self.y[1] + v[2] * self.z[1],
                v[0] * self.x[2] + v[1] * self.y[2] + v[2] * self.z[2],
            ]
        };
        let moved = rotate(child.origin);
        Self {
            origin: snap([
                self.origin[0] + moved[0],
                self.origin[1] + moved[1],
                self.origin[2] + moved[2],
            ]),
            x: snap(rotate(child.x)),
            y: snap(rotate(child.y)),
            z: snap(rotate(child.z)),
        }
    }

    /// Whether the axes are the parent's: the placement needs no `Axis`
    /// nor `RefDirection`.
    pub(super) fn is_unrotated(&self) -> bool {
        let near = |a: [f64; 3], b: [f64; 3]| a.iter().zip(b).all(|(a, b)| (a - b).abs() <= 1e-12);
        near(self.x, [1.0, 0.0, 0.0]) && near(self.z, [0.0, 0.0, 1.0])
    }
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn normalize(v: [f64; 3]) -> Option<[f64; 3]> {
    let length = dot(v, v).sqrt();
    (length.is_finite() && length > 1e-12).then(|| [v[0] / length, v[1] / length, v[2] / length])
}

/// Round away the noise a rotation leaves around exact values: a
/// coordinate within 1e-12 of an integer becomes that integer.
fn snap(v: [f64; 3]) -> [f64; 3] {
    v.map(|c| {
        let rounded = c.round();
        if (c - rounded).abs() <= 1e-12 {
            rounded + 0.0
        } else {
            c
        }
    })
}

#[cfg(test)]
mod tests {
    use super::Transform;

    #[test]
    fn composing_a_quarter_turn_moves_the_child_with_it() {
        let parent = Transform::from_axes([1.0, 0.0, 0.0], None, Some([0.0, 1.0, 0.0])).unwrap();
        let child = Transform::from_axes([1.0, 0.0, 0.0], None, None).unwrap();
        let grandchild = Transform::from_axes([0.0, 0.5, 0.0], None, None).unwrap();
        let composed = parent.then(&child).then(&grandchild);
        assert_eq!(composed.origin, [0.5, 1.0, 0.0]);
        assert_eq!(composed.x, [0.0, 1.0, 0.0]);
        assert_eq!(composed.z, [0.0, 0.0, 1.0]);
        assert!(!composed.is_unrotated());
        assert!(child.is_unrotated());
    }

    #[test]
    fn degenerate_axes_are_refused() {
        assert!(Transform::from_axes([0.0; 3], Some([0.0; 3]), None).is_none());
        assert!(Transform::from_axes([0.0; 3], None, Some([0.0, 0.0, 2.0])).is_none());
    }
}
