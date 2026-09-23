//! The model handle registry.
//!
//! Handles are globally unique, non-zero `u64` tokens drawn from a counter
//! that never reuses a value, so a stale handle can never alias a newer
//! model. Each model sits behind its own mutex: calls on different models
//! run in parallel, calls on one model are serialized.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, LazyLock, Mutex, MutexGuard, PoisonError};

use openbim_ifc_binding_core::{BindingError, IfcModel};

/// One live model plus the last error raised against it.
#[derive(Debug, Default)]
pub(crate) struct Entry {
    pub(crate) model: IfcModel,
    pub(crate) last_error: Option<BindingError>,
}

type Shared = Arc<Mutex<Entry>>;

static NEXT: AtomicU64 = AtomicU64::new(1);
static MODELS: LazyLock<Mutex<HashMap<u64, Shared>>> = LazyLock::new(Default::default);

/// Lock, recovering from poison: a panic is already reported to the caller
/// as `Panic`, and every mutation here is a single assignment, so the data
/// behind a poisoned lock is still consistent.
pub(crate) fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Register a model, returning its new handle.
pub(crate) fn insert(model: IfcModel) -> u64 {
    // `fetch_add` from 1 wraps only after 2^64 models, which is not reachable.
    let handle = NEXT.fetch_add(1, Ordering::Relaxed);
    let entry = Arc::new(Mutex::new(Entry {
        model,
        last_error: None,
    }));
    lock(&MODELS).insert(handle, entry);
    handle
}

/// The model behind `handle`, if it is live.
pub(crate) fn get(handle: u64) -> Option<Shared> {
    lock(&MODELS).get(&handle).cloned()
}

/// Remove `handle`; `false` if it was not live.
pub(crate) fn remove(handle: u64) -> bool {
    lock(&MODELS).remove(&handle).is_some()
}

/// How many models are live, for leak checks in tests and hosts.
pub(crate) fn live() -> usize {
    lock(&MODELS).len()
}
