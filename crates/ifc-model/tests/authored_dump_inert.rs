//! The coverage hook must be inert unless explicitly asked for.
//!
//! The gate runs `--all-features`, so this feature is compiled into
//! every gate run. If it wrote files whenever it was merely compiled
//! in, it would be a tax on every build rather than a tool someone
//! opts into.

#![cfg(feature = "authored-dump")]

use ifc_model::authored_dump;
use ifc_model::{Entity, Model, Value};

/// With `AUTHORED_DUMP` unset, recording is off and writes nothing.
///
/// This is the configuration every `--all-features` build runs in.
/// The assertion is on `authored_dump::recording()` rather than on an
/// absent directory: a hook that ignored the variable would still
/// leave any directory it was never told about missing, so checking
/// for absence proves nothing.
#[test]
fn the_hook_is_inert_without_the_environment_variable() {
    // During an actual audit the variable is set on purpose, and this
    // test has nothing to say about that case. Skipping keeps the tool
    // from failing the very run it is measuring.
    if std::env::var("AUTHORED_DUMP").is_ok() {
        return;
    }

    let mut model = Model::default();
    for _ in 0..64 {
        model.push(Entity::new("IFCWALL", vec![Value::Null; 9]));
    }

    assert!(
        !authored_dump::recording(),
        "the hook reports itself active with AUTHORED_DUMP unset",
    );
}
