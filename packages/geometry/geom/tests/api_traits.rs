//! Compile-time API ergonomics expected by downstream Rust clients.

use std::fmt::Debug;
#[cfg(feature = "model")]
use std::fmt::Display;
#[cfg(feature = "model")]
use std::hash::Hash;

fn value<T: Debug + Clone + PartialEq>() {}
#[cfg(feature = "model")]
fn id<T: Debug + Display + Copy + Eq + Ord + Hash>() {}
fn error<T: std::error::Error + Send + Sync + 'static>() {}

#[test]
fn default_surface_has_standard_traits() {
    value::<geom::Tolerance>();
    value::<geom::Aabb>();
    value::<geom::mesh::TriMesh>();
    error::<geom_core::ToleranceError>();
    error::<geom::mesh::MeshValidationError>();
}

#[test]
fn default_execution_errors_are_standard_errors() {
    error::<geom_backend_cpu::CpuConfigError>();
}

#[cfg(feature = "kernel")]
#[test]
fn kernel_errors_are_standard_errors() {
    error::<geom::kernel::GeomError>();
}

#[cfg(feature = "model")]
#[test]
fn model_handles_and_values_have_standard_traits() {
    id::<geom::model::NodeId>();
    value::<geom::model::GeometryGraph>();
    value::<geom::model::GeometryNode>();
}

#[cfg(feature = "profiles")]
#[test]
fn profile_values_are_debuggable_and_cloneable() {
    value::<geom::profile::Profile>();
}

#[cfg(feature = "curves")]
#[test]
fn curve_values_are_debuggable_and_cloneable() {
    value::<geom::curve::Curve2>();
    value::<geom::curve::Curve3>();
}

#[cfg(feature = "surfaces")]
#[test]
fn surface_values_are_debuggable_and_cloneable() {
    value::<geom::surface::Surface>();
}

#[cfg(feature = "topology")]
#[test]
fn topology_handles_are_typed_value_ids() {
    id::<geom::topology::VertexId>();
    id::<geom::topology::EdgeId>();
    id::<geom::topology::FaceId>();
}
