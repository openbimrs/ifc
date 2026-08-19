//! Portable CPU backend shell and honest unimplemented capability behavior.

use core::fmt;
use std::num::NonZeroUsize;
#[cfg(feature = "parallel")]
use std::sync::Arc;

use geom_kernel::{
    Backend, BackendDescriptor, BackendId, BooleanOp, ExecutionOptions, ExecutionTarget, GeomError,
    GeomResult, MeshBoolean, Operation,
};
use geom_mesh::TriMesh;

use crate::{CpuConfigError, CpuFeatures, CpuInstructionSet};

/// CPU backend configured for this process. Algorithm implementations land in
/// focused modules and advertise capabilities only when they are real.
#[derive(Clone)]
pub struct CpuBackend {
    instruction_set: CpuInstructionSet,
    features: CpuFeatures,
    threads: NonZeroUsize,
    #[cfg(feature = "parallel")]
    pool: Option<Arc<rayon::ThreadPool>>,
}

impl fmt::Debug for CpuBackend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CpuBackend")
            .field("instruction_set", &self.instruction_set)
            .field("features", &self.features)
            .field("threads", &self.threads)
            .finish_non_exhaustive()
    }
}

impl CpuBackend {
    /// Always-available portable backend.
    pub fn portable() -> Self {
        Self {
            instruction_set: CpuInstructionSet::Portable,
            features: CpuFeatures::detect(),
            threads: NonZeroUsize::MIN,
            #[cfg(feature = "parallel")]
            pool: None,
        }
    }

    /// Best instruction path compiled into this binary and available at runtime.
    pub fn detect() -> Self {
        let features = CpuFeatures::detect();
        #[cfg(feature = "simd")]
        let instruction_set = features.best();
        #[cfg(not(feature = "simd"))]
        let instruction_set = CpuInstructionSet::Portable;
        Self {
            instruction_set,
            features,
            threads: NonZeroUsize::MIN,
            #[cfg(feature = "parallel")]
            pool: None,
        }
    }

    pub(crate) fn from_configuration(
        instruction_set: CpuInstructionSet,
        features: CpuFeatures,
        threads: NonZeroUsize,
    ) -> Result<Self, CpuConfigError> {
        #[cfg(not(feature = "parallel"))]
        if threads.get() > 1 {
            return Err(CpuConfigError::ParallelFeatureDisabled);
        }

        #[cfg(feature = "parallel")]
        let pool = if threads.get() > 1 {
            Some(Arc::new(
                rayon::ThreadPoolBuilder::new()
                    .num_threads(threads.get())
                    .thread_name(|index| format!("geom-cpu-{index}"))
                    .build()
                    .map_err(|error| CpuConfigError::ThreadPool(error.to_string()))?,
            ))
        } else {
            None
        };

        Ok(Self {
            instruction_set,
            features,
            threads,
            #[cfg(feature = "parallel")]
            pool,
        })
    }

    /// Runtime-selected instruction set.
    pub const fn instruction_set(&self) -> CpuInstructionSet {
        self.instruction_set
    }

    /// Detected machine features.
    pub const fn features(&self) -> CpuFeatures {
        self.features
    }

    /// Configured worker bound.
    pub const fn thread_count(&self) -> NonZeroUsize {
        self.threads
    }

    /// Execute one closure inside this backend's local Rayon pool.
    #[cfg(feature = "parallel")]
    pub fn install<R: Send>(&self, operation: impl FnOnce() -> R + Send) -> R {
        match &self.pool {
            Some(pool) => pool.install(operation),
            None => operation(),
        }
    }

    fn id(&self) -> BackendId {
        BackendId::new(match self.instruction_set {
            CpuInstructionSet::Portable => "cpu-portable",
            CpuInstructionSet::Sse42 => "cpu-sse42",
            CpuInstructionSet::Avx2 => "cpu-avx2",
            CpuInstructionSet::Avx512 => "cpu-avx512",
            CpuInstructionSet::Neon => "cpu-neon",
        })
    }
}

impl Backend for CpuBackend {
    fn descriptor(&self) -> BackendDescriptor {
        BackendDescriptor {
            id: self.id(),
            target: if self.instruction_set == CpuInstructionSet::Portable {
                ExecutionTarget::PortableCpu
            } else {
                ExecutionTarget::OptimizedCpu
            },
            available: true,
            unavailable_reason: None,
            // Honest scaffold: detection and local scheduling exist, geometry
            // algorithms do not advertise support until implemented and tested.
            operations: Vec::new(),
        }
    }
}

impl MeshBoolean for CpuBackend {
    fn boolean(
        &self,
        left: &TriMesh,
        right: &TriMesh,
        _operation: BooleanOp,
        _options: &ExecutionOptions,
    ) -> GeomResult<TriMesh> {
        left.validate_structure()
            .map_err(|error| GeomError::InvalidInput(error.to_string()))?;
        right
            .validate_structure()
            .map_err(|error| GeomError::InvalidInput(error.to_string()))?;
        Err(GeomError::Unsupported {
            backend: self.id(),
            operation: Operation::MeshBoolean,
        })
    }
}

#[cfg(test)]
mod tests {
    use geom_core::{Tolerance, Vec3};

    use super::*;
    use crate::CpuBackendBuilder;

    #[test]
    fn portable_backend_is_always_constructible_and_honest() {
        let backend = CpuBackendBuilder::new().build().unwrap();
        assert!(backend.descriptor().available);
        assert!(backend.descriptor().operations.is_empty());
        let mesh = TriMesh::new(Vec::new(), Vec::new());
        let options = ExecutionOptions::new(Tolerance::METRE);
        assert!(matches!(
            backend.boolean(&mesh, &mesh, BooleanOp::Difference, &options),
            Err(GeomError::Unsupported { .. })
        ));
    }

    #[test]
    fn malformed_mesh_fails_before_unsupported_algorithm() {
        let backend = CpuBackendBuilder::new().build().unwrap();
        let bad = TriMesh::new(vec![Vec3::ZERO], vec![0, 1, 2]);
        let empty = TriMesh::default();
        let options = ExecutionOptions::new(Tolerance::METRE);
        assert!(matches!(
            backend.boolean(&bad, &empty, BooleanOp::Union, &options),
            Err(GeomError::InvalidInput(_))
        ));
    }
}
