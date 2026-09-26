//! Derive the published capability tables from the source that implements them.
//!
//! A hand-maintained capability matrix drifts, and a matrix that drifts is
//! worse than none: it is read as a promise (ADR 0005). The generated tables
//! live between sentinel comments in `docs/capabilities.md`; everything outside
//! them is prose a human owns.
//!
//! ## Internal split
//!
//! - `tables.rs`: the lowering tables, read from Rust source with `syn`.
//! - `census.rs`: the per-crate census and the scaffold count.
//! - `unhandled.rs`: schema-down walk for geometry items the source never names.

mod census;
mod tables;
mod unhandled;

use super::Output;
use crate::text::splice;
use crate::workspace::Workspace;

const TARGET: &str = "docs/capabilities.md";

pub(super) const IMPLEMENTED: &str = r#"<span class="status-implemented">Implemented</span>"#;
pub(super) const PLANNED: &str = r#"<span class="status-partial">Planned</span>"#;
pub(super) const PARTIAL: &str = r#"<span class="status-partial">Partial</span>"#;
pub(super) const ADMITTED: &str = r#"<span class="status-implemented">Admitted</span>"#;
pub(super) const REFUSED: &str = r#"<span class="status-partial">Refused</span>"#;

fn region(name: &str) -> (String, String) {
    (
        format!("<!-- CAPABILITIES:{name}:BEGIN -->"),
        format!("<!-- CAPABILITIES:{name}:END -->"),
    )
}

fn put(text: &str, name: &str, body: &str) -> Result<String, String> {
    let (begin, end) = region(name);
    splice(text, &begin, &end, body)
}

pub(super) fn generate(workspace: &Workspace) -> Result<Output, String> {
    let lowering = tables::Lowering::read(workspace)?;
    let unhandled = unhandled::table(workspace)?;
    Output::derive(workspace, TARGET, |current| {
        let census = census::table(workspace, current)?;
        let mut updated = put(current, "CENSUS", &census)?;
        updated = put(&updated, "UNHANDLED", &unhandled)?;
        updated = put(&updated, "GEOMETRY", &lowering.geometry_table())?;
        updated = put(&updated, "VARIANT", &lowering.variant_table())?;
        updated = put(&updated, "PROFILE", &lowering.profile_table())?;
        updated = put(&updated, "SCAFFOLDCOUNT", &census::scaffold_count(&census))?;
        Ok(updated)
    })
}
