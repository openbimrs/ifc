//! Scalar type and the tolerance policy.

/// The kernel stores coordinates in `f64`. IFC site coordinates (national grid
/// eastings/northings) routinely exceed 7 significant digits, which `f32`
/// cannot hold — storing them in `f32` loses millimetre precision at building
/// scale. Backends may compute in `f32` internally where they prove it safe.
pub type Scalar = f64;

/// An explicit tolerance, carried as a value.
///
/// BIM models arrive in millimetres *and* metres. A global epsilon is therefore
/// wrong in at least one of them, so every predicate that needs a tolerance
/// takes one. Construct from the model's length unit, do not hardcode.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tolerance {
    /// Absolute distance below which two positions are considered coincident,
    /// in the model's own length unit.
    pub linear: Scalar,
    /// Absolute angle in radians below which two directions are parallel.
    pub angular: Scalar,
}

impl Tolerance {
    /// Tolerance for a model whose length unit is **millimetres**.
    pub const MILLIMETRE: Self = Self {
        linear: 1e-3,
        angular: 1e-9,
    };
    /// Tolerance for a model whose length unit is **metres**.
    pub const METRE: Self = Self {
        linear: 1e-6,
        angular: 1e-9,
    };

    /// Are two scalars equal within the linear tolerance?
    #[inline]
    pub fn eq(&self, a: Scalar, b: Scalar) -> bool {
        (a - b).abs() <= self.linear
    }
}
