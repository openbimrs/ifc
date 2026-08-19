//! Backend-neutral geometry capability contracts.
//!
//! This crate contains traits, execution policy, capability descriptors, and
//! structured errors only. Concrete CPU/GPU implementations live in sibling
//! crates. That physical boundary prevents Cargo feature unification from
//! pulling a backend into `ifc-geometry` or another format adapter.

pub mod backend;
pub mod boolean;
pub mod capability;
pub mod catalog;
pub mod compile;
pub mod error;
pub mod execution;

pub use backend::Backend;
pub use boolean::{BooleanOp, MeshBoolean};
pub use capability::{
    BackendDescriptor, BackendId, ExecutionTarget, Operation, OperationSupport, Precision,
};
pub use catalog::BackendCatalog;
pub use compile::GeometryCompiler;
pub use error::{GeomError, GeomResult};
pub use execution::{Determinism, DevicePreference, ExecutionOptions, Parallelism};
