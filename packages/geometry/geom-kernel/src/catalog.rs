//! Descriptor catalog and policy filtering without concrete backend knowledge.

use crate::{
    Backend, BackendDescriptor, BackendId, Determinism, DevicePreference, ExecutionOptions,
    ExecutionTarget, Operation,
};

/// Runtime inventory populated by the application from whichever backend crates
/// it compiled. The contract crate never imports those crates itself.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BackendCatalog {
    descriptors: Vec<BackendDescriptor>,
}

impl BackendCatalog {
    /// Empty catalog.
    pub const fn new() -> Self {
        Self {
            descriptors: Vec::new(),
        }
    }

    /// Register or replace one backend descriptor by stable ID.
    pub fn register(&mut self, backend: &dyn Backend) {
        let descriptor = backend.descriptor();
        if let Some(existing) = self
            .descriptors
            .iter_mut()
            .find(|existing| existing.id == descriptor.id)
        {
            *existing = descriptor;
        } else {
            self.descriptors.push(descriptor);
        }
    }

    /// Registered descriptors in deterministic registration order.
    pub fn descriptors(&self) -> &[BackendDescriptor] {
        &self.descriptors
    }

    /// First compatible descriptor after explicit policy filtering.
    ///
    /// Ordering is application-controlled registration order. The catalog does
    /// not assume a GPU is always faster than a CPU; that requires benchmarks.
    pub fn select(
        &self,
        operation: Operation,
        work_items: usize,
        options: &ExecutionOptions,
    ) -> Option<&BackendDescriptor> {
        self.descriptors.iter().find(|descriptor| {
            if !device_matches(descriptor, options.device())
                || !descriptor.supports(operation, options.precision())
            {
                return false;
            }
            descriptor.operations.iter().any(|support| {
                support.operation == operation
                    && work_items >= support.minimum_batch_size
                    && (options.determinism() != Determinism::Required || support.deterministic)
            })
        })
    }

    /// Whether one backend is currently registered and available.
    pub fn is_available(&self, id: BackendId) -> bool {
        self.descriptors
            .iter()
            .any(|descriptor| descriptor.id == id && descriptor.available)
    }
}

fn device_matches(descriptor: &BackendDescriptor, preference: DevicePreference) -> bool {
    match preference {
        DevicePreference::Auto => true,
        DevicePreference::Cpu => matches!(
            descriptor.target,
            ExecutionTarget::PortableCpu | ExecutionTarget::OptimizedCpu
        ),
        DevicePreference::Gpu => descriptor.target == ExecutionTarget::Gpu,
        DevicePreference::Backend(id) => descriptor.id == id,
    }
}
