//! `cargo run -p xtask -- docs`: regenerate, or check, every generated docs region.
//!
//! Each generator reads the committed page, splices fresh content between
//! its sentinels, and returns the result. This module compares and writes, so
//! `--check` and regeneration can never disagree about what "current" means.
//!
//! ## Internal split
//!
//! - `changelog.rs`: `docs/project/changelog.md` from every crate's `CHANGELOG.md`.
//! - `capabilities.rs`: the generated tables of `docs/capabilities.md`.

mod capabilities;
mod changelog;

use std::path::PathBuf;

use crate::workspace::Workspace;

/// One generated file: where it lives and what it should contain.
pub(crate) struct Output {
    pub(crate) path: PathBuf,
    pub(crate) current: String,
    pub(crate) updated: String,
}

impl Output {
    /// Read `rel` under the workspace root and derive its updated content.
    pub(crate) fn derive(
        workspace: &Workspace,
        rel: &str,
        update: impl FnOnce(&str) -> Result<String, String>,
    ) -> Result<Self, String> {
        let path = workspace.root.join(rel);
        let current = std::fs::read_to_string(&path)
            .map_err(|error| format!("cannot read {rel}: {error}"))?;
        let updated = update(&current).map_err(|error| format!("{rel}: {error}"))?;
        Ok(Self {
            path,
            current,
            updated,
        })
    }
}

pub(crate) fn run(check: bool) -> Result<(), String> {
    let workspace = Workspace::load()?;
    let outputs = [
        changelog::generate(&workspace)?,
        capabilities::generate(&workspace)?,
    ];
    let stale: Vec<&Output> = outputs.iter().filter(|o| o.current != o.updated).collect();
    let relative = |output: &Output| {
        output
            .path
            .strip_prefix(&workspace.root)
            .unwrap_or(&output.path)
            .display()
            .to_string()
    };
    if check {
        if stale.is_empty() {
            println!("docs in sync ({} generated files)", outputs.len());
            return Ok(());
        }
        let names: Vec<String> = stale.iter().map(|o| relative(o)).collect();
        return Err(format!(
            "out of date: {}; run `cargo run -p xtask -- docs`",
            names.join(", ")
        ));
    }
    for output in &stale {
        std::fs::write(&output.path, &output.updated)
            .map_err(|error| format!("cannot write {}: {error}", relative(output)))?;
        println!("updated {}", relative(output));
    }
    if stale.is_empty() {
        println!("docs already in sync");
    }
    Ok(())
}
