//! One generated reference page per crate, the index that groups them, and
//! the crate map in `docs/architecture/crates.md`.
//!
//! Every fact on these pages is read from the crate itself: its manifest and
//! `[package.metadata.openbim]`, its crate-level docs, its `CHANGELOG.md`, and
//! the facade features that enable it. Nothing is typed twice.
//!
//! ## Internal split
//!
//! - `crate_docs.rs`: the crate-level `//!` docs as publishable Markdown.

pub(crate) mod crate_docs;

use std::collections::BTreeMap;

use super::capabilities::census::badge;
use super::changelog::absolutise;
use super::release::{self, Registry, Release};
use super::{generated_banner, Output};
use crate::text::splice;
use crate::workspace::{Crate, Workspace, GROUPS};

const TREE: &str = "https://github.com/openbimrs/ifc/tree/main";
const BLOB: &str = "https://github.com/openbimrs/ifc/blob/main";
const FACADE: &str = "openbim-ifc";
const CRATE_MAP: &str = "docs/architecture/crates.md";

/// Everything a crate page shows that is not on the manifest itself.
struct Facts {
    release: Option<Release>,
    latest_notes: Option<String>,
    registries: Vec<Registry>,
    /// Facade features that enable this crate.
    facade_features: Vec<String>,
}

pub(super) fn generate(workspace: &Workspace) -> Result<Vec<Output>, String> {
    let crates = workspace.crates()?;
    let facade = crates
        .iter()
        .find(|c| c.name == FACADE)
        .ok_or("the facade crate `openbim-ifc` is not in the workspace")?;
    let descriptions: BTreeMap<&str, &str> = crates
        .iter()
        .map(|c| (c.name.as_str(), c.description.as_str()))
        .collect();
    let feature_table = facade_features(facade, &descriptions)?;

    let mut facts = BTreeMap::new();
    for krate in &crates {
        let release = release::latest(workspace, krate)?;
        let latest_notes = match &release {
            Some(release) => section_body(workspace, krate, &release.version)?,
            None => None,
        };
        let facade_features = facade
            .features
            .iter()
            .filter(|(_, values)| values.iter().any(|v| *v == format!("dep:{}", krate.name)))
            .map(|(feature, _)| feature.clone())
            .collect();
        facts.insert(
            krate.name.clone(),
            Facts {
                release,
                latest_notes,
                registries: release::registries(workspace, krate),
                facade_features,
            },
        );
    }

    let map = crate_map(&crates);
    let mut outputs = vec![
        Output::whole(workspace, "docs/reference/index.md", index(&crates, &facts)),
        Output::derive(workspace, CRATE_MAP, |current| {
            splice(
                current,
                "<!-- CRATES:MAP:BEGIN -->",
                "<!-- CRATES:MAP:END -->",
                &map,
            )
        })?,
    ];
    for krate in &crates {
        let facts = &facts[&krate.name];
        let features = (krate.name == FACADE).then_some(feature_table.as_str());
        let page = page(krate, facts, features)?;
        let rel = format!("docs/reference/crates/{}.md", krate.name);
        outputs.push(Output::whole(workspace, &rel, page));
    }
    Ok(outputs)
}

fn index(crates: &[Crate], facts: &BTreeMap<String, Facts>) -> String {
    let mut out = vec![
        generated_banner().to_owned(),
        String::new(),
        "# Crate reference".to_owned(),
        String::new(),
        format!(
            "The family is {} crates, each versioned and released on its own. Most \
             applications depend on the [`openbim-ifc`](./crates/openbim-ifc) facade and \
             enable the features they need; the crates below are what those features \
             pull in.",
            crates.len()
        ),
        String::new(),
        "Every page here is generated from the crate itself: its manifest, its crate \
         documentation and its changelog."
            .to_owned(),
    ];
    for (group, title) in GROUPS {
        let members: Vec<&Crate> = crates.iter().filter(|c| c.group == *group).collect();
        if members.is_empty() {
            continue;
        }
        out.push(String::new());
        out.push(format!("## {title}"));
        out.push(String::new());
        out.push("| Crate | Status | Latest release | Description |".to_owned());
        out.push("| --- | --- | --- | --- |".to_owned());
        for krate in members {
            let released = facts[&krate.name]
                .release
                .as_ref()
                .map_or_else(|| "not released".to_owned(), |r| r.version.clone());
            out.push(format!(
                "| [`{name}`](./crates/{name}) | {} | {released} | {} |",
                badge(&krate.status),
                krate.description,
                name = krate.name
            ));
        }
    }
    out.push(String::new());
    out.join("\n")
}

/// Every crate by group, with the workspace crates it depends on.
fn crate_map(crates: &[Crate]) -> String {
    let mut out = Vec::new();
    for (group, title) in GROUPS {
        let members: Vec<&Crate> = crates.iter().filter(|c| c.group == *group).collect();
        if members.is_empty() {
            continue;
        }
        out.push(format!("### {title}"));
        out.push(String::new());
        out.push("| Crate | Status | Depends on | Description |".to_owned());
        out.push("| --- | --- | --- | --- |".to_owned());
        for krate in members {
            let deps: Vec<String> = krate
                .internal_deps
                .iter()
                .map(|dep| format!("[`{dep}`](/reference/crates/{dep})"))
                .collect();
            out.push(format!(
                "| [`{name}`](/reference/crates/{name}) | {} | {} | {} |",
                badge(&krate.status),
                if krate.name == FACADE {
                    "the core, domain and geometry crates, each behind a feature".to_owned()
                } else if deps.is_empty() {
                    "—".to_owned()
                } else {
                    deps.join(", ")
                },
                krate.description,
                name = krate.name
            ));
        }
        out.push(String::new());
    }
    out.join("\n").trim_end().to_owned()
}

fn page(krate: &Crate, facts: &Facts, facade_table: Option<&str>) -> Result<String, String> {
    let mut out = vec![
        "---".to_owned(),
        "editLink: false".to_owned(),
        "---".to_owned(),
        String::new(),
        generated_banner().to_owned(),
        String::new(),
        format!("# {}", krate.name),
        String::new(),
        krate.description.clone(),
        String::new(),
        "| | |".to_owned(),
        "| --- | --- |".to_owned(),
        format!("| Status | {} |", badge(&krate.status)),
    ];
    match &facts.release {
        Some(release) => {
            out.push(format!(
                "| Latest release | {} ({}) |",
                release.version, release.date
            ));
            if release.version != krate.version {
                out.push(format!("| On `main` | {} (unreleased) |", krate.version));
            }
        }
        None => out.push(format!(
            "| Latest release | not released (`main` is {}) |",
            krate.version
        )),
    }
    let registries: Vec<String> = facts
        .registries
        .iter()
        .map(|r| format!("[{} `{}`]({})", r.kind, r.package, r.url))
        .collect();
    out.push(format!(
        "| Registries | {} |",
        if registries.is_empty() {
            "not published yet; build from source".to_owned()
        } else {
            registries.join(" · ")
        }
    ));
    if !facts.facade_features.is_empty() {
        let features: Vec<String> = facts
            .facade_features
            .iter()
            .map(|f| format!("`{f}`"))
            .collect();
        out.push(format!(
            "| Via the facade | [`openbim-ifc`](./openbim-ifc) feature {} |",
            features.join(", ")
        ));
    }
    let mut api = Vec::new();
    if let Some((lib, _)) = &krate.lib {
        api.push(format!("[rustdoc](/ifc/api/rustdoc/{lib}/index.html)"));
    }
    if krate.publish && facts.release.is_some() {
        api.push(format!("[docs.rs](https://docs.rs/{})", krate.name));
    }
    if !api.is_empty() {
        out.push(format!("| API documentation | {} |", api.join(" · ")));
    }
    out.push(format!(
        "| Source | [`{dir}/`]({TREE}/{dir}) |",
        dir = krate.dir
    ));

    if let Some((_, path)) = &krate.lib {
        let overview = crate_docs::overview(path)?;
        if !overview.is_empty() {
            out.extend([
                String::new(),
                "## Overview".to_owned(),
                String::new(),
                overview,
            ]);
        }
    }
    if let Some(table) = facade_table {
        out.extend([
            String::new(),
            "## Features".to_owned(),
            String::new(),
            table.to_owned(),
        ]);
    } else if !krate.features.is_empty() {
        out.extend([
            String::new(),
            "## Features".to_owned(),
            String::new(),
            "| Feature | Default | Enables |".to_owned(),
            "| --- | --- | --- |".to_owned(),
        ]);
        for (feature, values) in &krate.features {
            let default = if krate.default_features.contains(feature) {
                "yes"
            } else {
                ""
            };
            let enables: Vec<String> = values.iter().map(|v| format!("`{v}`")).collect();
            out.push(format!(
                "| `{feature}` | {default} | {} |",
                enables.join(", ")
            ));
        }
    }
    if !krate.internal_deps.is_empty() {
        out.extend([String::new(), "## Depends on".to_owned(), String::new()]);
        for dep in &krate.internal_deps {
            out.push(format!("- [`{dep}`](./{dep})"));
        }
    }
    out.extend([String::new(), "## Changes".to_owned(), String::new()]);
    if let (Some(release), Some(notes)) = (&facts.release, &facts.latest_notes) {
        out.push(format!(
            "Latest release, {} ({}):",
            release.version, release.date
        ));
        out.push(String::new());
        out.push(notes.clone());
        out.push(String::new());
    }
    out.push(format!(
        "Full history: [`{dir}/CHANGELOG.md`]({BLOB}/{dir}/CHANGELOG.md)",
        dir = krate.dir
    ));
    out.push(String::new());
    Ok(out.join("\n"))
}

/// The facade's feature table: every feature, described.
///
/// A feature that enables exactly one crate is described by that crate's
/// `description`; any other needs a line in the facade's
/// `[package.metadata.openbim.features]`. A feature with neither, or a line
/// for a feature that no longer exists, fails the build.
fn facade_features(facade: &Crate, descriptions: &BTreeMap<&str, &str>) -> Result<String, String> {
    for noted in facade.feature_notes.keys() {
        if !facade.features.contains_key(noted) {
            return Err(format!(
                "openbim-ifc: [package.metadata.openbim.features] describes `{noted}`, \
                 which is not a feature"
            ));
        }
    }
    let mut out = vec![
        "| Feature | Default | What it enables |".to_owned(),
        "| --- | --- | --- |".to_owned(),
    ];
    for (feature, values) in &facade.features {
        let deps: Vec<&str> = values
            .iter()
            .filter_map(|v| v.strip_prefix("dep:"))
            .collect();
        let description = match (facade.feature_notes.get(feature), deps.as_slice()) {
            (Some(note), _) => note.clone(),
            (None, [dep]) => {
                let text = descriptions.get(dep).copied().unwrap_or_default();
                format!("[`{dep}`](./{dep}): {text}")
            }
            _ => {
                return Err(format!(
                    "openbim-ifc: feature `{feature}` needs a description in \
                     [package.metadata.openbim.features]"
                ))
            }
        };
        let default = if facade.default_features.contains(feature) {
            "yes"
        } else {
            ""
        };
        out.push(format!("| `{feature}` | {default} | {description} |"));
    }
    Ok(out.join("\n"))
}

/// The body of one release section, links absolutised for the docs site.
fn section_body(
    workspace: &Workspace,
    krate: &Crate,
    version: &str,
) -> Result<Option<String>, String> {
    let path = workspace.root.join(&krate.dir).join("CHANGELOG.md");
    let text = std::fs::read_to_string(&path)
        .map_err(|error| format!("{}/CHANGELOG.md: {error}", krate.dir))?;
    let mut body = Vec::new();
    let mut inside = false;
    for line in crate::text::splitlines(&text) {
        if let Some((heading, _)) = super::changelog::heading(line) {
            inside = heading == version;
            continue;
        }
        if inside && !line.starts_with('[') {
            body.push(line);
        }
    }
    let body = body.join("\n").trim().to_owned();
    Ok((!body.is_empty()).then(|| absolutise(&body)))
}
