//! What a backend can do, and on what hardware.
//!
//! The dispatcher needs to answer "can this backend run here, and is it worth
//! choosing?" **before** handing it work. Capability reporting is how a backend
//! declines gracefully instead of failing at call time.

/// Which execution strategy a backend uses.
///
/// This is deliberately an open-ended, ordered notion of "more specialized":
/// [`Backend::Scalar`] must always be available as the correctness reference
/// that every other backend is differentially tested against.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Backend {
    /// Portable scalar CPU. Always available. The correctness oracle.
    Scalar,
    /// SIMD-accelerated CPU (SSE/AVX2/AVX-512 chosen at runtime).
    Simd,
    /// GPU offload. Only worth it above a work-size threshold — see
    /// [`Capabilities::gpu_threshold_triangles`].
    Gpu,
}

impl Backend {
    /// Stable string for logs and differential-test reports.
    pub fn as_str(&self) -> &'static str {
        match self {
            Backend::Scalar => "scalar",
            Backend::Simd => "simd",
            Backend::Gpu => "gpu",
        }
    }
}

/// What a backend supports on the current machine.
#[derive(Debug, Clone, PartialEq)]
pub struct Capabilities {
    /// Which strategy this backend implements.
    pub backend: Backend,
    /// Is it usable on THIS machine right now? A SIMD backend compiled for
    /// AVX-512 reports `false` on a machine without it; a GPU backend reports
    /// `false` when no device is present. The dispatcher must never select a
    /// backend that reports `false` here.
    pub available: bool,
    /// Whether mesh boolean is implemented by this backend.
    pub mesh_boolean: bool,
    /// Below this triangle count, dispatching to this backend is not worth the
    /// setup cost (PCIe transfer for GPU, or none for CPU backends). `None`
    /// means no threshold — always worth using when available.
    pub gpu_threshold_triangles: Option<usize>,
}

impl Capabilities {
    /// The baseline every workspace build must be able to construct: portable
    /// scalar CPU, always available, no threshold.
    pub fn scalar_baseline() -> Self {
        Self {
            backend: Backend::Scalar,
            available: true,
            mesh_boolean: true,
            gpu_threshold_triangles: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scalar_is_always_available_and_is_the_least_specialized() {
        let c = Capabilities::scalar_baseline();
        assert!(c.available);
        assert_eq!(c.backend, Backend::Scalar);
        assert!(Backend::Scalar < Backend::Simd);
        assert!(Backend::Simd < Backend::Gpu);
    }
}
