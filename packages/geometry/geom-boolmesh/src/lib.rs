#![forbid(unsafe_code)]

//! `boolmesh`-backed [`geom_kernel::MeshBoolean`] provider (ADR 0014).

mod convert;
mod provider;

pub use provider::BoolmeshBoolean;
