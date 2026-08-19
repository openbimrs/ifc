//! Backend capability vocabulary used for explicit, inspectable dispatch.

use core::fmt;

/// Stable backend identifier for logs, configuration, and differential tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BackendId(&'static str);

impl BackendId {
    /// Construct from a process-static identifier.
    pub const fn new(value: &'static str) -> Self {
        Self(value)
    }

    /// Identifier string.
    pub const fn as_str(self) -> &'static str {
        self.0
    }
}

impl fmt::Display for BackendId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

/// Broad execution target; custom accelerators remain first-class.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ExecutionTarget {
    /// Portable scalar CPU path.
    PortableCpu,
    /// Runtime-selected CPU specialization.
    OptimizedCpu,
    /// General-purpose GPU compute.
    Gpu,
    /// Other accelerator supplied by a downstream crate.
    Accelerator,
}

/// Geometry capability that can be selected independently.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Operation {
    /// Curve position/derivative evaluation.
    CurveEvaluation,
    /// Surface position/normal evaluation.
    SurfaceEvaluation,
    /// Profile boolean or normalization.
    ProfileOperation,
    /// Sweep/extrusion/revolution construction.
    Sweep,
    /// Exact or approximate tessellation.
    Tessellation,
    /// Mesh boolean.
    MeshBoolean,
    /// Spatial broad/narrow phase query.
    SpatialQuery,
    /// Area, volume, length, or centroid measurement.
    Measurement,
    /// Explicit diagnosis or repair.
    Healing,
    /// Batched affine transform.
    BatchTransform,
    /// Compile a complete geometry graph.
    GraphCompilation,
}

/// Numeric precision a backend can honor for an operation.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Precision {
    /// IEEE 754 binary64 throughout correctness-sensitive work.
    F64,
    /// IEEE 754 binary32; caller must accept reduced coordinate precision.
    F32,
    /// Backend chooses per stage and documents the error bound.
    Mixed,
}

/// Support statement for one operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationSupport {
    /// Operation.
    pub operation: Operation,
    /// Supported precision modes.
    pub precision: Vec<Precision>,
    /// Whether deterministic output ordering is available.
    pub deterministic: bool,
    /// Work-item count below which this backend should not be auto-selected.
    /// This is backend data, not a global guessed GPU threshold.
    pub minimum_batch_size: usize,
}

/// Runtime facts and implemented operations for one backend instance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackendDescriptor {
    /// Stable identity.
    pub id: BackendId,
    /// Hardware class.
    pub target: ExecutionTarget,
    /// Whether the required hardware/runtime is usable now.
    pub available: bool,
    /// Human-readable reason when unavailable.
    pub unavailable_reason: Option<String>,
    /// Capabilities implemented by this backend.
    pub operations: Vec<OperationSupport>,
}

impl BackendDescriptor {
    /// Whether the backend advertises one operation and precision.
    pub fn supports(&self, operation: Operation, precision: Precision) -> bool {
        self.available
            && self.operations.iter().any(|support| {
                support.operation == operation && support.precision.contains(&precision)
            })
    }
}
