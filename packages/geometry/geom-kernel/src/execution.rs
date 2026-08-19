//! Execution policy passed explicitly to every costly operation.

use geom_core::Tolerance;

use crate::{BackendId, Precision};

/// Reproducibility requirement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Determinism {
    /// Same inputs and options must produce stable ordering and values.
    Required,
    /// Backend may use faster nondeterministic scheduling.
    BestEffort,
}

/// CPU scheduling preference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Parallelism {
    /// One worker.
    Serial,
    /// Backend chooses from available parallelism and workload size.
    Auto,
    /// Upper bound on worker count. Zero is rejected by the builder.
    Threads(usize),
}

/// Device selection preference. `Auto` is a policy request, not permission to
/// silently reduce precision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DevicePreference {
    /// Select from compatible registered backends.
    Auto,
    /// Require a CPU backend.
    Cpu,
    /// Require a GPU backend.
    Gpu,
    /// Require one named backend.
    Backend(BackendId),
}

/// Operation policy with explicit tolerance and precision.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExecutionOptions {
    tolerance: Tolerance,
    precision: Precision,
    determinism: Determinism,
    parallelism: Parallelism,
    device: DevicePreference,
    memory_budget_bytes: Option<usize>,
}

impl ExecutionOptions {
    /// Start from the required model-aware tolerance.
    pub const fn new(tolerance: Tolerance) -> Self {
        Self {
            tolerance,
            precision: Precision::F64,
            determinism: Determinism::Required,
            parallelism: Parallelism::Auto,
            device: DevicePreference::Auto,
            memory_budget_bytes: None,
        }
    }

    /// Set required precision.
    pub const fn with_precision(mut self, precision: Precision) -> Self {
        self.precision = precision;
        self
    }

    /// Set determinism requirement.
    pub const fn with_determinism(mut self, value: Determinism) -> Self {
        self.determinism = value;
        self
    }

    /// Set scheduling preference. Returns `None` for zero explicit threads.
    pub const fn with_parallelism(mut self, value: Parallelism) -> Option<Self> {
        if matches!(value, Parallelism::Threads(0)) {
            return None;
        }
        self.parallelism = value;
        Some(self)
    }

    /// Set device preference.
    pub const fn with_device(mut self, value: DevicePreference) -> Self {
        self.device = value;
        self
    }

    /// Bound temporary allocation.
    pub const fn with_memory_budget(mut self, bytes: usize) -> Self {
        self.memory_budget_bytes = Some(bytes);
        self
    }

    /// Tolerance.
    pub const fn tolerance(self) -> Tolerance {
        self.tolerance
    }

    /// Precision.
    pub const fn precision(self) -> Precision {
        self.precision
    }

    /// Determinism requirement.
    pub const fn determinism(self) -> Determinism {
        self.determinism
    }

    /// Scheduling preference.
    pub const fn parallelism(self) -> Parallelism {
        self.parallelism
    }

    /// Device preference.
    pub const fn device(self) -> DevicePreference {
        self.device
    }

    /// Optional temporary-memory budget.
    pub const fn memory_budget_bytes(self) -> Option<usize> {
        self.memory_budget_bytes
    }
}
