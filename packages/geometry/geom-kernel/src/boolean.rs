//! Mesh boolean capability and executable provider registry.

use std::sync::Arc;

use geom_core::BooleanOperator;
use geom_mesh::TriMesh;

use crate::{
    Backend, BackendId, DevicePreference, ExecutionOptions, ExecutionTarget, GeomError, GeomResult,
    Operation,
};

/// Mesh boolean provider.
///
/// Implementing this trait is the capability declaration. Providers that do not
/// implement mesh booleans must not implement this trait.
pub trait MeshBoolean: Backend {
    /// Apply one set operation.
    fn boolean(
        &self,
        subject: &TriMesh,
        tool: &TriMesh,
        operation: BooleanOperator,
        options: &ExecutionOptions,
    ) -> GeomResult<TriMesh>;

    /// Subtract many tools in one batch so implementations can union or schedule
    /// cutters efficiently. The default is correct but deliberately simple.
    fn subtract_many(
        &self,
        subject: &TriMesh,
        tools: &[TriMesh],
        options: &ExecutionOptions,
    ) -> GeomResult<TriMesh> {
        let mut result = subject.clone();
        for tool in tools {
            result = self.boolean(&result, tool, BooleanOperator::Difference, options)?;
        }
        Ok(result)
    }
}

#[derive(Debug, Clone)]
struct RegisteredBoolean {
    priority: i32,
    provider: Arc<dyn MeshBoolean>,
}

/// Ordered executable providers for one narrow operation.
///
/// Fallback happens only for `Unsupported` or `Unavailable`; numerical and data
/// failures are returned immediately rather than hidden by another algorithm.
#[derive(Debug, Clone, Default)]
pub struct MeshBooleanRegistry {
    providers: Vec<RegisteredBoolean>,
}

impl MeshBooleanRegistry {
    /// Empty registry.
    pub const fn new() -> Self {
        Self {
            providers: Vec::new(),
        }
    }

    /// Register an implementation. Higher priorities run first.
    pub fn register<B>(&mut self, priority: i32, provider: B)
    where
        B: MeshBoolean + 'static,
    {
        self.register_arc(priority, Arc::new(provider));
    }

    /// Register a shared trait object.
    pub fn register_arc(&mut self, priority: i32, provider: Arc<dyn MeshBoolean>) {
        self.providers
            .push(RegisteredBoolean { priority, provider });
        self.providers
            .sort_by_key(|entry| std::cmp::Reverse(entry.priority));
    }

    /// Registered providers in dispatch order.
    pub fn providers(&self) -> impl Iterator<Item = &dyn MeshBoolean> {
        self.providers.iter().map(|entry| entry.provider.as_ref())
    }

    /// Execute according to device policy with narrow fallback semantics.
    pub fn boolean(
        &self,
        subject: &TriMesh,
        tool: &TriMesh,
        operation: BooleanOperator,
        options: &ExecutionOptions,
    ) -> GeomResult<TriMesh> {
        let mut last_retryable = None;
        for entry in &self.providers {
            let descriptor = entry.provider.descriptor();
            if !matches_device(options.device(), descriptor.id, descriptor.target) {
                continue;
            }
            match entry.provider.boolean(subject, tool, operation, options) {
                Ok(mesh) => return Ok(mesh),
                Err(error @ (GeomError::Unsupported { .. } | GeomError::Unavailable { .. })) => {
                    last_retryable = Some(error);
                }
                Err(error) => return Err(error),
            }
        }
        Err(last_retryable.unwrap_or(GeomError::Unsupported {
            backend: BackendId::new("mesh-boolean-registry"),
            operation: Operation::MeshBoolean,
        }))
    }
}

fn matches_device(preference: DevicePreference, id: BackendId, target: ExecutionTarget) -> bool {
    match preference {
        DevicePreference::Auto => true,
        DevicePreference::Cpu => {
            matches!(
                target,
                ExecutionTarget::PortableCpu | ExecutionTarget::OptimizedCpu
            )
        }
        DevicePreference::Gpu => matches!(target, ExecutionTarget::Gpu),
        DevicePreference::Backend(required) => required == id,
    }
}

#[cfg(test)]
mod tests {
    use geom_core::Tolerance;

    use super::*;
    use crate::BackendDescriptor;

    #[derive(Debug)]
    struct EchoBoolean {
        id: BackendId,
        target: ExecutionTarget,
    }

    impl Backend for EchoBoolean {
        fn descriptor(&self) -> BackendDescriptor {
            BackendDescriptor {
                id: self.id,
                target: self.target,
            }
        }
    }

    impl MeshBoolean for EchoBoolean {
        fn boolean(
            &self,
            subject: &TriMesh,
            _tool: &TriMesh,
            _operation: BooleanOperator,
            _options: &ExecutionOptions,
        ) -> GeomResult<TriMesh> {
            Ok(subject.clone())
        }
    }

    #[test]
    fn registry_stores_executable_traits_not_capability_flags() {
        let mut registry = MeshBooleanRegistry::new();
        registry.register(
            10,
            EchoBoolean {
                id: BackendId::new("echo"),
                target: ExecutionTarget::PortableCpu,
            },
        );
        let options = ExecutionOptions::new(Tolerance::METRE);
        let mesh = TriMesh::default();
        assert_eq!(
            registry
                .boolean(&mesh, &mesh, BooleanOperator::Difference, &options)
                .expect("registered provider executes"),
            mesh
        );
    }
}
