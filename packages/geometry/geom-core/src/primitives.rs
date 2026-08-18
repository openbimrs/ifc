//! Positions, transforms, and bounds. Data only.

use crate::scalar::Scalar;

/// Double-precision 3D vector. Re-exported from `glam`, which provides SIMD
/// acceleration in pure Rust — no C++ dependency.
pub type Vec3 = glam::DVec3;

/// Double-precision 4x4 affine transform (IFC `IfcObjectPlacement` lowers here).
pub type Mat4 = glam::DAffine3;

/// Axis-aligned bounding box. The broad-phase currency of the whole kernel.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

impl Aabb {
    /// An empty box that absorbs any point on the first [`Aabb::extend`].
    pub fn empty() -> Self {
        Self {
            min: Vec3::splat(Scalar::INFINITY),
            max: Vec3::splat(Scalar::NEG_INFINITY),
        }
    }

    /// Grow to include `p`.
    #[inline]
    pub fn extend(&mut self, p: Vec3) {
        self.min = self.min.min(p);
        self.max = self.max.max(p);
    }

    /// Does this box overlap `other`? Touching counts as overlap.
    #[inline]
    pub fn intersects(&self, other: &Self) -> bool {
        self.min.x <= other.max.x
            && self.max.x >= other.min.x
            && self.min.y <= other.max.y
            && self.max.y >= other.min.y
            && self.min.z <= other.max.z
            && self.max.z >= other.min.z
    }

    /// True when no point has ever been added.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.min.x > self.max.x
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_box_is_empty_and_absorbs_first_point() {
        let mut b = Aabb::empty();
        assert!(b.is_empty());
        b.extend(Vec3::new(1.0, 2.0, 3.0));
        assert!(!b.is_empty());
        assert_eq!(b.min, b.max);
    }

    #[test]
    fn disjoint_boxes_do_not_intersect() {
        let mut a = Aabb::empty();
        a.extend(Vec3::ZERO);
        a.extend(Vec3::splat(1.0));
        let mut b = Aabb::empty();
        b.extend(Vec3::splat(2.0));
        b.extend(Vec3::splat(3.0));
        assert!(!a.intersects(&b));
        assert!(a.intersects(&a));
    }
}
