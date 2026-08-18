//! Hardware backends and runtime selection.
//!
//! This module is the **only** place that knows every backend exists.
//! `packages/ifc/` depends on `geom-kernel` with `default-features = false`, so
//! none of this code is even compiled into the IFC layer. That containment is
//! what keeps the kernel swappable: replacing the geometry implementation means
//! providing a different `MeshBoolean` impl, not editing the IFC layer.
//!
//! # Selection policy
//!
//! Most specialized backend that is (a) available on this machine and (b)
//! actually implements the requested operation, with scalar as the guaranteed
//! floor. A backend reporting `available: false` is never selected, so a binary
//! carrying AVX-512 paths still runs correctly on a machine without them.
//!
//! # Features
//!
//! | Feature | Default | Effect |
//! |---|---|---|
//! | `scalar` | yes | portable reference implementation, the correctness oracle |
//! | `simd` | yes | runtime-detected AVX2/AVX-512 paths |
//! | `gpu` | **no** | GPU offload; pulls a driver stack, see `docs/adr/0002` |
//!
//! With no features at all you get the traits and nothing else — the intended
//! configuration for a consumer supplying a foreign kernel.

use crate::capability::{Backend, Capabilities};

#[cfg(feature = "gpu")]
pub mod gpu;
#[cfg(feature = "scalar")]
pub mod scalar;
#[cfg(feature = "simd")]
pub mod simd;

#[cfg(feature = "gpu")]
pub use gpu::GpuBackend;
#[cfg(feature = "scalar")]
pub use scalar::ScalarBackend;
#[cfg(feature = "simd")]
pub use simd::{SimdBackend, SimdLevel};

/// A survey of what this machine can do, gathered once at startup.
#[derive(Debug, Clone, Default)]
pub struct Dispatcher {
    caps: Vec<Capabilities>,
}

impl Dispatcher {
    /// Probe every compiled-in backend on the current machine.
    //
    // Built incrementally because each push is feature-gated; `vec![]` cannot
    // express conditional elements, hence the allow.
    #[allow(unused_mut, clippy::vec_init_then_push)]
    pub fn detect() -> Self {
        let mut caps: Vec<Capabilities> = Vec::new();

        #[cfg(feature = "scalar")]
        caps.push(ScalarBackend::new().capabilities());

        #[cfg(feature = "simd")]
        caps.push(SimdBackend::detect().capabilities());

        #[cfg(feature = "gpu")]
        caps.push(match GpuBackend::detect() {
            Some(gpu) => gpu.capabilities(),
            None => GpuBackend.capabilities(),
        });

        Self { caps }
    }

    /// Everything probed, including unavailable backends (useful for a
    /// `--capabilities` diagnostic dump).
    pub fn capabilities(&self) -> &[Capabilities] {
        &self.caps
    }

    /// The most specialized backend that is available and implements mesh
    /// boolean.
    ///
    /// Returns `None` when no backend implements it yet — which is the honest
    /// answer in the current scaffold, and is why this returns an `Option`
    /// rather than silently handing back a backend that will fail at call time.
    pub fn best_for_mesh_boolean(&self) -> Option<Backend> {
        self.caps
            .iter()
            .filter(|c| c.available && c.mesh_boolean)
            .map(|c| c.backend)
            .max()
    }

    /// Whether a backend was detected as usable here.
    pub fn is_available(&self, backend: Backend) -> bool {
        self.caps
            .iter()
            .any(|c| c.backend == backend && c.available)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(feature = "scalar")]
    fn scalar_is_always_available() {
        assert!(Dispatcher::detect().is_available(Backend::Scalar));
    }

    #[test]
    fn probes_every_compiled_in_backend() {
        let expected = cfg!(feature = "scalar") as usize
            + cfg!(feature = "simd") as usize
            + cfg!(feature = "gpu") as usize;
        assert_eq!(Dispatcher::detect().capabilities().len(), expected);
    }

    #[test]
    fn admits_no_boolean_backend_rather_than_returning_a_broken_one() {
        // Honest scaffold state: nothing implements boolean yet.
        assert_eq!(Dispatcher::detect().best_for_mesh_boolean(), None);
    }

    #[test]
    #[cfg(feature = "gpu")]
    fn gpu_is_not_selected_when_unavailable() {
        assert!(!Dispatcher::detect().is_available(Backend::Gpu));
    }
}
