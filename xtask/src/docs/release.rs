//! What each crate has released, and where it can be installed from.
//!
//! The released version is the newest dated section of the crate's own
//! `CHANGELOG.md`. The release commit is what dates that section, so this is a
//! property of the commit, not of the moment the docs are built: deriving it
//! from git tags instead would make every release (the tag is pushed after the
//! release commit merges) turn `main`'s docs stale and fail every open PR.
//! Whether each release is actually tagged is a separate check.

use super::changelog::{compare_versions, heading};
use crate::text::splitlines;
use crate::workspace::{Crate, Workspace};

/// The newest release a crate's changelog records.
pub(crate) struct Release {
    pub(crate) version: String,
    pub(crate) date: String,
}

/// A registry a crate is published to.
pub(crate) struct Registry {
    /// `crates.io`, `npm` or `PyPI`.
    pub(crate) kind: &'static str,
    /// The package name there, which need not equal the crate name.
    pub(crate) package: String,
    pub(crate) url: String,
}

/// The newest dated `## [x.y.z] - date` section, if any.
pub(crate) fn latest(workspace: &Workspace, krate: &Crate) -> Result<Option<Release>, String> {
    let path = workspace.root.join(&krate.dir).join("CHANGELOG.md");
    let text = std::fs::read_to_string(&path)
        .map_err(|error| format!("{}/CHANGELOG.md: {error}", krate.dir))?;
    let newest = splitlines(&text)
        .into_iter()
        .filter_map(heading)
        .filter(|(version, date)| !date.is_empty() && !version.eq_ignore_ascii_case("unreleased"))
        .max_by(|a, b| compare_versions(&a.0, &b.0));
    Ok(newest.map(|(version, date)| Release { version, date }))
}

/// Registries the crate is published to: crates.io unless `publish = false`,
/// plus npm and PyPI when their manifests exist. These are the same files the
/// release workflow publishes from (`scripts/release-crate.py`).
///
/// Empty for a crate that has never been released: a manifest that *could*
/// publish is not a package anyone can install, and a link to it would 404.
pub(crate) fn registries(workspace: &Workspace, krate: &Crate) -> Vec<Registry> {
    let mut out = Vec::new();
    if latest(workspace, krate).ok().flatten().is_none() {
        return out;
    }
    if krate.publish {
        out.push(Registry {
            kind: "crates.io",
            package: krate.name.clone(),
            url: format!("https://crates.io/crates/{}", krate.name),
        });
    }
    let dir = workspace.root.join(&krate.dir);
    if let Some(name) = std::fs::read_to_string(dir.join("npm/package.json"))
        .ok()
        .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
        .and_then(|json| json.get("name")?.as_str().map(str::to_owned))
    {
        out.push(Registry {
            kind: "npm",
            url: format!("https://www.npmjs.com/package/{name}"),
            package: name,
        });
    }
    if let Some(name) = std::fs::read_to_string(dir.join("pyproject.toml"))
        .ok()
        .and_then(|text| project_name(&text))
    {
        out.push(Registry {
            kind: "PyPI",
            url: format!("https://pypi.org/project/{name}/"),
            package: name,
        });
    }
    out
}

/// `name = "…"` in the `[project]` table of a `pyproject.toml`.
fn project_name(text: &str) -> Option<String> {
    let mut in_project = false;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_project = line == "[project]";
            continue;
        }
        if !in_project {
            continue;
        }
        if let Some(value) = line.strip_prefix("name") {
            let value = value.trim_start().strip_prefix('=')?.trim();
            return Some(value.trim_matches('"').to_owned());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pyproject_name_is_read_from_project_table() {
        let text = "[build-system]\nname = \"x\"\n[project]\nname = \"openbim-ifc\"\n";
        assert_eq!(project_name(text).as_deref(), Some("openbim-ifc"));
        assert_eq!(project_name("[tool]\nname = \"y\"\n"), None);
    }
}
