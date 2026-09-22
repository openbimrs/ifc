//! Coverage audit: record which entity types are actually constructed.
//!
//! Off unless the `authored-dump` feature is on *and* `AUTHORED_DUMP`
//! names a directory, so an `--all-features` build pays one relaxed
//! atomic load per entity and nothing else.
//!
//! # Why this exists
//!
//! Static scanning of writer call sites cannot answer "is this entity
//! authorable". A type name reaches [`crate::Entity::new`] through a
//! const, a catalogue row, or a match arm returning it into a tuple, so
//! a scan both misses real writers and counts names that are only
//! mentioned. Recording what is built during a test run answers it
//! directly.
//!
//! # Origin matters
//!
//! [`crate::Transaction::create`] is the authoring path a writer goes
//! through. [`crate::Model::push`] is what test fixtures use to stand
//! up input. Counting both as authored reports an entity as covered
//! when only a fixture ever built one, so the origin is recorded and
//! the caller filters on it.
//!
//! # Usage
//!
//! ```text
//! AUTHORED_DUMP=/tmp/dump \
//!   cargo test --workspace --all-features --features ifc-model/authored-dump
//! python3 scripts/authored-coverage.py /tmp/dump
//! ```

use std::collections::BTreeSet;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Mutex, OnceLock};

/// Where records accumulate until the process exits.
fn seen() -> &'static Mutex<BTreeSet<(&'static str, String)>> {
    static SEEN: OnceLock<Mutex<BTreeSet<(&'static str, String)>>> = OnceLock::new();
    SEEN.get_or_init(|| Mutex::new(BTreeSet::new()))
}

/// Tri-state cache of the `AUTHORED_DUMP` lookup: 0 unknown, 1 off, 2 on.
///
/// The variable is read once. Without this every constructed entity pays
/// an environment lookup and a `String` allocation, which is a real cost
/// on a full `--all-features` run where the feature is on but the
/// variable is unset.
static STATE: AtomicU8 = AtomicU8::new(0);

/// The directory named by `AUTHORED_DUMP`, resolved once.
fn directory() -> Option<&'static str> {
    static DIR: OnceLock<Option<String>> = OnceLock::new();
    DIR.get_or_init(|| std::env::var("AUTHORED_DUMP").ok())
        .as_deref()
}

/// Whether recording is on, reading the environment at most once.
///
/// Public so a test can assert the hook stays inert when the feature
/// is compiled in but `AUTHORED_DUMP` is unset -- the configuration
/// every `--all-features` build runs in. Asserting on an absent output
/// directory would pass even if this returned `true`, since the writer
/// has no directory to write to either way.
pub fn recording() -> bool {
    match STATE.load(Ordering::Relaxed) {
        1 => false,
        2 => true,
        _ => {
            let on = directory().is_some();
            STATE.store(if on { 2 } else { 1 }, Ordering::Relaxed);
            on
        }
    }
}

/// Record one constructed type name against the path that built it.
///
/// `origin` is `"create"` for the authoring path and `"push"` for direct
/// insertion. Names are upper-cased because the crates disagree on the
/// casing they store and the audit compares against the schema.
pub fn record(type_name: &str, origin: &'static str) {
    if !recording() {
        return;
    }
    let Ok(mut set) = seen().lock() else {
        return;
    };
    if !set.insert((origin, type_name.to_ascii_uppercase())) {
        return;
    }
    // Flushed on growth, not on every call: a test binary has no exit
    // hook to write from, and the set grows at most once per distinct
    // (origin, type) pair -- hundreds of writes across a run, not one
    // per constructed entity.
    flush(&set);
}

/// Write the accumulated set to a per-process file.
///
/// Per-process because cargo runs test binaries in parallel and a single
/// shared path would have them overwrite each other. The reader unions
/// every file in the directory.
///
/// Failures are ignored: this is an audit aid, and a full disk or a
/// missing directory must not fail the run it is observing.
fn flush(set: &BTreeSet<(&'static str, String)>) {
    let Some(dir) = directory() else {
        return;
    };
    let body = set
        .iter()
        .map(|(origin, name)| format!("{origin}\t{name}"))
        .collect::<Vec<_>>()
        .join("\n");
    let path = std::path::Path::new(dir).join(format!("{}.txt", std::process::id()));
    let _ = std::fs::create_dir_all(dir);
    let _ = std::fs::write(path, body);
}
