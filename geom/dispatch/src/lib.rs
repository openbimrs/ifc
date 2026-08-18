//! `geom-dispatch` — pick the best available backend at runtime.
//!
//! This is the **only** crate that knows all backends exist. `ifc/` depends on
//! `geom-kernel` traits and (optionally) this selector; it never names
//! `geom-cpu`, `geom-simd`, or `geom-gpu`. That containment is what keeps the
//! kernel swappable: replacing the geometry implementation means providing a
//! different selector, not editing the IFC layer.
//!
//! # Selection policy
//!
//! Most specialized backend that is (a) available on this machine and (b)
//! actually implements the requested operation, with scalar as the guaranteed
//! floor. A backend that reports `available: false` is never selected, so a
//! binary built with AVX-512 paths still runs correctly on a machine without
//! them.

use geom_kernel::{Backend, Capabilities};

pub use geom_cpu::ScalarBackend;
pub use geom_gpu::GpuBackend;
pub use geom_simd::{SimdBackend, SimdLevel};

/// A survey of what this machine can do, gathered once at startup.
#[derive(Debug, Clone)]
pub struct Dispatcher {
    caps: Vec<Capabilities>,
}

impl Dispatcher {
    /// Probe every backend on the current machine.
    pub fn detect() -> Self {
        let mut caps = vec![ScalarBackend::new().capabilities()];
        caps.push(SimdBackend::detect().capabilities());
        if let Some(gpu) = GpuBackend::detect() {
            caps.push(gpu.capabilities());
        } else {
            caps.push(GpuBackend.capabilities());
        }
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
    fn scalar_is_always_available() {
        assert!(Dispatcher::detect().is_available(Backend::Scalar));
    }

    #[test]
    fn probes_all_three_backends() {
        assert_eq!(Dispatcher::detect().capabilities().len(), 3);
    }

    #[test]
    fn admits_no_boolean_backend_rather_than_returning_a_broken_one() {
        // Honest scaffold state: nothing implements boolean yet.
        assert_eq!(Dispatcher::detect().best_for_mesh_boolean(), None);
    }

    #[test]
    fn gpu_is_not_selected_when_unavailable() {
        assert!(!Dispatcher::detect().is_available(Backend::Gpu));
    }
}
