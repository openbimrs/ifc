//! Backend identity metadata. Operation traits remain the sole capability truth.

use core::fmt;

/// Stable provider identifier for logs and explicit selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BackendId(&'static str);

impl BackendId {
    /// Construct an identifier owned by the provider implementation.
    pub const fn new(value: &'static str) -> Self {
        Self(value)
    }

    /// Identifier text.
    pub const fn as_str(self) -> &'static str {
        self.0
    }
}

impl fmt::Display for BackendId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

/// Broad execution target. Specific ISA/device features stay in provider crates.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExecutionTarget {
    /// Portable scalar CPU implementation.
    PortableCpu,
    /// Runtime-selected CPU implementation.
    OptimizedCpu,
    /// General-purpose GPU compute.
    Gpu,
    /// Other accelerator supplied downstream.
    Accelerator,
}

/// Arithmetic precision accepted or required by an operation.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Precision {
    /// IEEE single precision.
    F32,
    /// IEEE double precision.
    F64,
    /// Deliberate mixed-precision path with documented error bounds.
    Mixed,
}

/// Operation name used for diagnostics only. Implementing an operation trait is
/// the capability proof; this enum never drives capability discovery.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Operation {
    CurveEvaluation,
    SurfaceEvaluation,
    ProfileTriangulation,
    Sweep,
    Tessellation,
    MeshBoolean,
    SpatialQuery,
    Measurement,
    Healing,
    GraphCompilation,
}

/// Provider identity only. It deliberately contains no operation booleans.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackendDescriptor {
    /// Stable implementation identity.
    pub id: BackendId,
    /// Hardware class used by execution policy.
    pub target: ExecutionTarget,
}

impl BackendDescriptor {
    /// Construct provider identity metadata.
    pub const fn new(id: BackendId, target: ExecutionTarget) -> Self {
        Self { id, target }
    }
}
