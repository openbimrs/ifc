//! `docs/.vitepress/data/facts.json`: every number or version prose may quote.
//!
//! A page that states a count ("31 crates") or a version drifts the day it is
//! written. Pages read these values instead, and the site config reads the
//! crate list for the reference sidebar, so nothing here is typed twice.

use super::release;
use super::Output;
use crate::workspace::{Workspace, GROUPS, STATUSES};

pub(super) fn generate(workspace: &Workspace) -> Result<Output, String> {
    let crates = workspace.crates()?;
    let mut per_crate = serde_json::Map::new();
    for krate in &crates {
        let latest = release::latest(workspace, krate)?;
        let registries: Vec<serde_json::Value> = release::registries(workspace, krate)
            .into_iter()
            .map(|r| serde_json::json!({ "registry": r.kind, "package": r.package, "url": r.url }))
            .collect();
        per_crate.insert(
            krate.name.clone(),
            serde_json::json!({
                "description": krate.description,
                "group": krate.group,
                "status": krate.status,
                "version": krate.version,
                "released": latest.as_ref().map(|r| r.version.clone()),
                "released_date": latest.as_ref().map(|r| r.date.clone()),
                "registries": registries,
            }),
        );
    }
    let mut by_status = serde_json::Map::new();
    for status in STATUSES {
        let count = crates.iter().filter(|c| c.status == *status).count();
        by_status.insert((*status).to_owned(), count.into());
    }
    let groups: Vec<serde_json::Value> = GROUPS
        .iter()
        .map(|(key, title)| {
            let members: Vec<&str> = crates
                .iter()
                .filter(|c| c.group == *key)
                .map(|c| c.name.as_str())
                .collect();
            serde_json::json!({ "key": key, "title": title, "crates": members })
        })
        .collect();
    let facts = serde_json::json!({
        "crates": {
            "total": crates.len(),
            "by_status": by_status,
            "groups": groups,
        },
        "crate": per_crate,
    });
    let mut json = serde_json::to_string_pretty(&facts).map_err(|error| error.to_string())?;
    json.push('\n');
    Ok(Output::whole(
        workspace,
        "docs/.vitepress/data/facts.json",
        json,
    ))
}
