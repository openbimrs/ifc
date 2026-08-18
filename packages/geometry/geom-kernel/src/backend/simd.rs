//! SIMD CPU backend with **runtime** feature detection.
//!
//! # Why runtime detection, not `target-cpu=native`
//!
//! The workspace currently builds with `-C target-cpu=native`, which is right
//! for a dev box we control but produces a binary that crashes with SIGILL on
//! any older machine. A library meant to be built upon cannot ship that.
//!
//! The intended design is the standard portable pattern: compile the specialized
//! paths with `#[target_feature(enable = "avx2")]` / `"avx512f"`, detect support
//! once at startup with `is_x86_feature_detected!`, and select. That yields a
//! single portable binary that still uses AVX-512 where present — which is what
//! [`SimdBackend::detect`] is for.
//!
//! # Where SIMD actually pays in this workspace
//!
//! Not everywhere. The wins are in the wide, regular, data-parallel passes:
//! transforming vertex buffers by a placement matrix, computing per-triangle
//! AABBs, broad-phase AABB overlap tests, and triangle-triangle intersection
//! over batches. Topological work (half-edge walks, boolean face classification)
//! is branchy and pointer-chasing — SIMD does not help there, and claiming it
//! would be dishonest.
//!
//! # Status
//!
//! Detection is implemented and tested. The accelerated kernels are not yet
//! written; the backend reports `mesh_boolean: false` until they are.

use crate::{Backend, Capabilities};

/// Which x86-64 SIMD width is usable on this machine, detected at runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SimdLevel {
    /// No SIMD beyond the SSE2 baseline guaranteed by x86-64.
    Baseline,
    /// AVX2 (256-bit).
    Avx2,
    /// AVX-512 foundation + doubleword/quadword (512-bit).
    Avx512,
}

/// SIMD CPU backend.
#[derive(Debug, Clone, Copy)]
pub struct SimdBackend {
    level: SimdLevel,
}

impl SimdBackend {
    /// Detect the best SIMD level available on the **current** machine.
    ///
    /// On non-x86-64 targets this returns [`SimdLevel::Baseline`]; an ARM/NEON
    /// path would extend this function rather than any caller.
    pub fn detect() -> Self {
        #[cfg(target_arch = "x86_64")]
        {
            if std::is_x86_feature_detected!("avx512f") && std::is_x86_feature_detected!("avx512dq")
            {
                return Self {
                    level: SimdLevel::Avx512,
                };
            }
            if std::is_x86_feature_detected!("avx2") {
                return Self {
                    level: SimdLevel::Avx2,
                };
            }
        }
        Self {
            level: SimdLevel::Baseline,
        }
    }

    /// The detected level.
    pub fn level(&self) -> SimdLevel {
        self.level
    }

    /// Capabilities on this machine. Reports unavailable when no SIMD beyond
    /// the baseline exists, so the dispatcher falls back to scalar instead of
    /// paying dispatch overhead for nothing.
    pub fn capabilities(&self) -> Capabilities {
        Capabilities {
            backend: Backend::Simd,
            available: self.level > SimdLevel::Baseline,
            mesh_boolean: false,
            gpu_threshold_triangles: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detection_runs_and_agrees_with_itself() {
        let a = SimdBackend::detect();
        let b = SimdBackend::detect();
        assert_eq!(a.level(), b.level(), "detection must be deterministic");
    }

    #[test]
    fn availability_tracks_detected_level() {
        let b = SimdBackend::detect();
        assert_eq!(b.capabilities().available, b.level() > SimdLevel::Baseline);
    }
}
