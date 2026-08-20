//! Rigid transforms: the composition algebra placements reduce to.
//!
//! IFC expresses position as nested `IfcAxis2Placement3D` inside
//! `IfcLocalPlacement` chains, plus `IfcCartesianTransformationOperator` for
//! mapped items. All of it collapses to a 4x3 affine transform, which is what
//! this module provides.
//!
//! # Why 4x3 and not 4x4
//!
//! The bottom row of an IFC transform is always `[0,0,0,1]`: there is no
//! projective component. Storing it would invite code that reads it, and a
//! non-affine transform in a building model is always a bug.
//!
//! Non-uniform scale IS representable, because
//! `IfcCartesianTransformationOperator3DnonUniform` exists.

/// An affine transform: a 3x3 linear part plus a translation.
///
/// Column-major: `basis[i]` is the image of basis vector `i`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform {
    /// Images of the X, Y, Z basis vectors.
    pub basis: [[f64; 3]; 3],
    /// Translation applied after the linear part.
    pub origin: [f64; 3],
}

impl Default for Transform {
    fn default() -> Self {
        Self::identity()
    }
}

impl Transform {
    /// The identity transform.
    pub const fn identity() -> Self {
        Self {
            basis: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            origin: [0.0, 0.0, 0.0],
        }
    }

    /// A pure translation.
    pub const fn translation(origin: [f64; 3]) -> Self {
        Self {
            basis: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            origin,
        }
    }

    /// Build from an origin and axis directions, Gram-Schmidt orthonormalized.
    ///
    /// IFC gives `Axis` (local Z) and `RefDirection` (approximate local X) and
    /// explicitly allows them to be non-perpendicular: the spec derives X by
    /// projecting `RefDirection` onto the plane normal to `Axis`. Skipping
    /// that projection produces a sheared transform that looks almost right,
    /// which is worse than looking obviously wrong.
    ///
    /// Returns `None` if the axes are degenerate (zero-length or parallel).
    pub fn from_axes(
        origin: [f64; 3],
        axis: Option<[f64; 3]>,
        ref_direction: Option<[f64; 3]>,
    ) -> Option<Self> {
        let z = normalize(axis.unwrap_or([0.0, 0.0, 1.0]))?;
        let reference = ref_direction.unwrap_or_else(|| default_ref_direction(z));

        // Project the reference direction onto the plane normal to z.
        let dot = dot(reference, z);
        let projected = [
            reference[0] - dot * z[0],
            reference[1] - dot * z[1],
            reference[2] - dot * z[2],
        ];
        let x = normalize(projected)?;
        let y = cross(z, x);

        Some(Self {
            basis: [x, y, z],
            origin,
        })
    }

    /// Apply this transform to a point.
    pub fn apply(&self, p: [f64; 3]) -> [f64; 3] {
        [
            self.basis[0][0] * p[0]
                + self.basis[1][0] * p[1]
                + self.basis[2][0] * p[2]
                + self.origin[0],
            self.basis[0][1] * p[0]
                + self.basis[1][1] * p[1]
                + self.basis[2][1] * p[2]
                + self.origin[1],
            self.basis[0][2] * p[0]
                + self.basis[1][2] * p[1]
                + self.basis[2][2] * p[2]
                + self.origin[2],
        ]
    }

    /// Apply the linear part only, without translating.
    ///
    /// Correct for directions and normals under rigid transforms. Under
    /// non-uniform scale, normals need the inverse transpose; that is the
    /// kernel's concern, not ours.
    pub fn apply_direction(&self, v: [f64; 3]) -> [f64; 3] {
        [
            self.basis[0][0] * v[0] + self.basis[1][0] * v[1] + self.basis[2][0] * v[2],
            self.basis[0][1] * v[0] + self.basis[1][1] * v[1] + self.basis[2][1] * v[2],
            self.basis[0][2] * v[0] + self.basis[1][2] * v[1] + self.basis[2][2] * v[2],
        ]
    }

    /// Convert to the format-neutral geometry transform at the IFC boundary.
    pub fn to_geom(self) -> axiolid_core::Transform3 {
        let columns = self.basis.map(axiolid_core::Vec3::from_array);
        axiolid_core::Transform3::from_mat3_translation(
            axiolid_core::Mat3::from_cols(columns[0], columns[1], columns[2]),
            axiolid_core::Vec3::from_array(self.origin),
        )
    }

    /// Compose: `self` applied after `inner`.
    ///
    /// This is the operation a placement chain folds with. Order matters and
    /// getting it backwards places every child relative to the wrong parent,
    /// so the convention is stated here once: `parent.compose(&child)` yields
    /// the child's world transform.
    pub fn compose(&self, inner: &Transform) -> Transform {
        Transform {
            basis: [
                self.apply_direction(inner.basis[0]),
                self.apply_direction(inner.basis[1]),
                self.apply_direction(inner.basis[2]),
            ],
            origin: self.apply(inner.origin),
        }
    }

    /// Scale the linear part uniformly, e.g. for a transformation operator.
    pub fn scaled(&self, factor: f64) -> Transform {
        Transform {
            basis: [
                scale(self.basis[0], factor),
                scale(self.basis[1], factor),
                scale(self.basis[2], factor),
            ],
            origin: self.origin,
        }
    }

    /// Scale each axis independently, for the non-uniform operator.
    pub fn scaled_nonuniform(&self, factors: [f64; 3]) -> Transform {
        Transform {
            basis: [
                scale(self.basis[0], factors[0]),
                scale(self.basis[1], factors[1]),
                scale(self.basis[2], factors[2]),
            ],
            origin: self.origin,
        }
    }

    /// Convert the translation to metres, leaving the basis dimensionless.
    ///
    /// IFC coordinates carry the file's length unit; direction ratios and
    /// scale factors do not. Scaling the basis as well would compound the
    /// unit into every rotation and silently resize geometry, so only the
    /// origin is converted. Apply this exactly once, at the boundary where a
    /// source frame becomes project space.
    pub fn to_metres(self, units: &crate::units::UnitScale) -> Transform {
        Transform {
            basis: self.basis,
            origin: self.origin.map(|coordinate| units.length(coordinate)),
        }
    }

    /// Is this within tolerance of the identity?
    pub fn is_identity(&self, tolerance: f64) -> bool {
        let id = Transform::identity();
        self.origin
            .iter()
            .zip(id.origin)
            .all(|(a, b)| (a - b).abs() <= tolerance)
            && self
                .basis
                .iter()
                .flatten()
                .zip(id.basis.iter().flatten())
                .all(|(a, b)| (a - b).abs() <= tolerance)
    }
}

/// A sensible local X when `RefDirection` is omitted.
///
/// The spec says the default is the projection of the global X axis; when the
/// local Z *is* global X, that degenerates, so global Z is used instead.
fn default_ref_direction(z: [f64; 3]) -> [f64; 3] {
    if z[0].abs() > 0.9 {
        [0.0, 0.0, 1.0]
    } else {
        [1.0, 0.0, 0.0]
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

fn scale(v: [f64; 3], f: f64) -> [f64; 3] {
    [v[0] * f, v[1] * f, v[2] * f]
}

/// Normalize, or `None` if the vector is too short to have a direction.
fn normalize(v: [f64; 3]) -> Option<[f64; 3]> {
    let len = dot(v, v).sqrt();
    if len < 1e-12 {
        return None;
    }
    Some([v[0] / len, v[1] / len, v[2] / len])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: [f64; 3], b: [f64; 3]) -> bool {
        a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-9)
    }

    #[test]
    fn identity_leaves_points_alone() {
        assert!(close(
            Transform::identity().apply([1.0, 2.0, 3.0]),
            [1.0, 2.0, 3.0]
        ));
    }

    #[test]
    fn translation_moves_points_but_not_directions() {
        let t = Transform::translation([10.0, 0.0, 0.0]);
        assert!(close(t.apply([1.0, 0.0, 0.0]), [11.0, 0.0, 0.0]));
        assert!(
            close(t.apply_direction([1.0, 0.0, 0.0]), [1.0, 0.0, 0.0]),
            "a direction must not be translated"
        );
    }

    /// The spec allows RefDirection to be non-perpendicular to Axis and
    /// requires projecting it. Skipping that yields a sheared basis.
    #[test]
    fn non_perpendicular_ref_direction_is_projected_not_used_raw() {
        let t = Transform::from_axes(
            [0.0, 0.0, 0.0],
            Some([0.0, 0.0, 1.0]),
            Some([1.0, 0.0, 0.5]), // deliberately not perpendicular to Z
        )
        .unwrap();

        assert!(
            close(t.basis[0], [1.0, 0.0, 0.0]),
            "X must be projected into the plane normal to Z, got {:?}",
            t.basis[0]
        );
        assert!(
            (dot(t.basis[0], t.basis[2])).abs() < 1e-12,
            "basis must be orthogonal"
        );
    }

    #[test]
    fn axes_default_to_the_global_frame() {
        let t = Transform::from_axes([0.0, 0.0, 0.0], None, None).unwrap();
        assert!(t.is_identity(1e-12));
    }

    #[test]
    fn degenerate_axes_are_rejected_rather_than_producing_nonsense() {
        assert!(Transform::from_axes([0.0; 3], Some([0.0, 0.0, 0.0]), None).is_none());
        // RefDirection parallel to Axis leaves nothing to project.
        assert!(
            Transform::from_axes([0.0; 3], Some([0.0, 0.0, 1.0]), Some([0.0, 0.0, 1.0])).is_none()
        );
    }

    /// A storey at z=3 containing a wall at z=1 puts the wall at z=4.
    #[test]
    fn composition_stacks_translations() {
        let storey = Transform::translation([0.0, 0.0, 3.0]);
        let wall = Transform::translation([0.0, 0.0, 1.0]);
        assert!(close(storey.compose(&wall).origin, [0.0, 0.0, 4.0]));
    }

    /// Composition is not commutative; the convention must hold.
    #[test]
    fn composition_applies_rotation_to_the_child_offset() {
        // Parent rotated 90 degrees about Z.
        let parent = Transform {
            basis: [[0.0, 1.0, 0.0], [-1.0, 0.0, 0.0], [0.0, 0.0, 1.0]],
            origin: [0.0, 0.0, 0.0],
        };
        let child = Transform::translation([1.0, 0.0, 0.0]);
        let world = parent.compose(&child);
        assert!(
            close(world.origin, [0.0, 1.0, 0.0]),
            "child X offset must rotate into parent Y, got {:?}",
            world.origin
        );
    }

    #[test]
    fn non_uniform_scale_is_representable() {
        let t = Transform::identity().scaled_nonuniform([2.0, 3.0, 4.0]);
        assert!(close(t.apply([1.0, 1.0, 1.0]), [2.0, 3.0, 4.0]));
    }
}
