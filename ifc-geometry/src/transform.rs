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

    /// [`Self::from_axes`], falling back to the identity transform when the
    /// axes are absent or degenerate.
    ///
    /// IFC's `Axis`/`RefDirection` are both optional and, per spec, default to
    /// the standard basis when omitted — the common case, so most call sites
    /// that use `from_axes` immediately follow it with
    /// `.unwrap_or_else(Transform::identity)`. This collapses that
    /// boilerplate. Reach for `from_axes` directly when a degenerate axis
    /// pair should be a reportable error instead of a silent identity.
    pub fn from_axes_or_identity(
        origin: [f64; 3],
        axis: Option<[f64; 3]>,
        ref_direction: Option<[f64; 3]>,
    ) -> Self {
        Self::from_axes(origin, axis, ref_direction).unwrap_or(Self {
            basis: Self::identity().basis,
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
    /// Correct for vectors and tangents. Surface normals under non-uniform scale
    /// must use `apply_unit_normal` instead.
    pub fn apply_direction(&self, v: [f64; 3]) -> [f64; 3] {
        [
            self.basis[0][0] * v[0] + self.basis[1][0] * v[1] + self.basis[2][0] * v[2],
            self.basis[0][1] * v[0] + self.basis[1][1] * v[1] + self.basis[2][1] * v[2],
            self.basis[0][2] * v[0] + self.basis[1][2] * v[1] + self.basis[2][2] * v[2],
        ]
    }

    /// Transform a surface normal as a covector and return unit length.
    ///
    /// The cofactor form is equivalent to inverse-transpose multiplication but
    /// avoids explicitly inverting the affine basis. Scaling the basis first
    /// prevents finite IFC operators from overflowing the cross products.
    #[cfg(feature = "lowering")]
    pub(crate) fn apply_unit_normal(&self, normal: [f64; 3]) -> Option<[f64; 3]> {
        let scale = self
            .basis
            .iter()
            .flatten()
            .map(|value| value.abs())
            .fold(0.0_f64, f64::max);
        if !scale.is_finite() || scale == 0.0 || normal.iter().any(|value| !value.is_finite()) {
            return None;
        }
        let [a, b, c] = self.basis.map(|axis| axis.map(|value| value / scale));
        let [bc, ca, ab] = [cross(b, c), cross(c, a), cross(a, b)];
        let determinant = dot(a, bc);
        if !determinant.is_finite() || determinant == 0.0 {
            return None;
        }
        let sign = determinant.signum();
        normalize([
            sign * (bc[0] * normal[0] + ca[0] * normal[1] + ab[0] * normal[2]),
            sign * (bc[1] * normal[0] + ca[1] * normal[1] + ab[1] * normal[2]),
            sign * (bc[2] * normal[0] + ca[2] * normal[1] + ab[2] * normal[2]),
        ])
    }

    /// Convert to a neutral orthonormal frame at the IFC boundary.
    ///
    /// `axiolid_core::Frame3` is structurally public and cannot enforce unit,
    /// orthogonal, right-handed axes itself. This method is therefore the one
    /// executable IFC-to-Axiolid frame contract. It rejects non-finite origins,
    /// scaled or sheared axes, and mirrored frames before neutral construction.
    /// IFC placements reach this method after `Axis`/`RefDirection` have been
    /// normalized and Gram-Schmidt orthogonalized; mapped-item scale stays on
    /// an `Instance` transform and must never leak into a surface frame.
    #[cfg(feature = "lowering")]
    pub fn to_geom_frame(
        self,
        entity: ifc_model::EntityId,
    ) -> crate::GeometryResult<axiolid_core::Frame3> {
        const INVARIANT_EPSILON: f64 = 1e-9;

        let finite_origin = self.origin.iter().all(|value| value.is_finite());
        let finite_basis = self.basis.iter().flatten().all(|value| value.is_finite());
        if !finite_origin || !finite_basis {
            return Err(crate::GeometryError::Degenerate {
                entity,
                type_name: "IfcAxis2Placement".to_string(),
                detail: "neutral frame origin and axes must be finite".to_string(),
            });
        }

        let [x, y, z] = self.basis;
        let unit = [dot(x, x), dot(y, y), dot(z, z)]
            .into_iter()
            .all(|length_squared| (length_squared - 1.0).abs() <= INVARIANT_EPSILON);
        let orthogonal = dot(x, y).abs() <= INVARIANT_EPSILON
            && dot(x, z).abs() <= INVARIANT_EPSILON
            && dot(y, z).abs() <= INVARIANT_EPSILON;
        let right_handed = dot(cross(x, y), z) >= 1.0 - INVARIANT_EPSILON;
        if !unit || !orthogonal || !right_handed {
            return Err(crate::GeometryError::Degenerate {
                entity,
                type_name: "IfcAxis2Placement".to_string(),
                detail: "neutral frame axes must be unit, orthogonal, and right-handed".to_string(),
            });
        }

        Ok(axiolid_core::Frame3 {
            origin: axiolid_core::Point3::from_array(self.origin),
            x: axiolid_core::Vec3::from_array(x),
            y: axiolid_core::Vec3::from_array(y),
            z: axiolid_core::Vec3::from_array(z),
        })
    }

    /// Convert to the format-neutral geometry transform at the IFC boundary.
    #[cfg(feature = "lowering")]
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

/// Normalize, or `None` if the vector is zero or non-finite.
fn normalize(v: [f64; 3]) -> Option<[f64; 3]> {
    let scale = v
        .iter()
        .map(|component| component.abs())
        .fold(0.0, f64::max);
    if scale == 0.0 || !scale.is_finite() {
        return None;
    }
    let scaled = [v[0] / scale, v[1] / scale, v[2] / scale];
    let length = dot(scaled, scaled).sqrt();
    Some([scaled[0] / length, scaled[1] / length, scaled[2] / length])
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

    #[cfg(feature = "lowering")]
    #[test]
    fn neutral_frames_enforce_the_ifc_to_axiolid_axis_invariant() {
        let id = ifc_model::EntityId(7);
        assert!(Transform::identity().to_geom_frame(id).is_ok());

        let invalid = [
            Transform {
                basis: [[2.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
                origin: [0.0; 3],
            },
            Transform {
                basis: [[1.0, 0.0, 0.0], [0.5, 1.0, 0.0], [0.0, 0.0, 1.0]],
                origin: [0.0; 3],
            },
            Transform {
                basis: [[1.0, 0.0, 0.0], [0.0, -1.0, 0.0], [0.0, 0.0, 1.0]],
                origin: [0.0; 3],
            },
            Transform {
                basis: Transform::identity().basis,
                origin: [f64::NAN, 0.0, 0.0],
            },
        ];

        for transform in invalid {
            let error = transform
                .to_geom_frame(id)
                .expect_err("invalid axes must not reach axiolid_core::Frame3");
            assert!(
                matches!(error, crate::GeometryError::Degenerate { entity, .. } if entity == id)
            );
        }
    }
}
