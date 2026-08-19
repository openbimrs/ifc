//! CPU backend shell with runtime ISA detection and optional local Rayon pool.
//!
//! Default build is portable and single-threaded. `simd` compiles specialized
//! paths but still selects them at runtime. `parallel` creates a backend-owned
//! pool instead of mutating Rayon's global pool.

pub mod backend;
pub mod config;
pub mod features;

pub use backend::CpuBackend;
pub use config::{CpuBackendBuilder, CpuConfigError, InstructionPolicy};
pub use features::{CpuFeatures, CpuInstructionSet};
