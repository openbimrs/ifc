//! `docs/coverage.md`: what the crates measurably cover, from their own data.
//!
//! Every table here is read from a machine-checked source:
//!
//! - `docs/.vitepress/data/authored-coverage.json`, which `scripts/gate.sh`
//!   refreshes from its own test run (every concrete IFC4X3 entity a writer
//!   produced);
//! - the geometry declaration, disposition and WHERE-rule manifests in
//!   `ifc-geometry/data/`, which `ifc-geometry/tests/declaration_manifest.rs`
//!   asserts against the schema;
//! - `ifc-style`'s `APPEARANCE_DECLARATIONS` and `ifc-validate`'s `RULES`,
//!   read as Rust.

use std::collections::BTreeMap;

use super::Output;
use crate::rust_source::{self, Consts};
use crate::text::splice;
use crate::workspace::Workspace;

const PAGE: &str = "docs/coverage.md";
const AUTHORED: &str = "docs/.vitepress/data/authored-coverage.json";
const SUPPORT: &str = "ifc-geometry/data/ifc4-add2-tc1-geometry-support.tsv";
const DISPOSITIONS: &str = "ifc-geometry/data/ifc4-representation-item-dispositions.tsv";
const WHERE_RULES: &str = "ifc-geometry/data/ifc4-where-rules.tsv";
const APPEARANCE: &str = "ifc-style/src/coverage.rs";
const VALIDATION: &str = "ifc-validate/src/where_rule/registry.rs";

pub(super) fn generate(workspace: &Workspace) -> Result<Output, String> {
    let regions = [
        ("AUTHORED", authored(workspace)?),
        ("GEOMETRY", geometry(workspace)?),
        ("DISPOSITIONS", dispositions(workspace)?),
        ("PRESENTATION", presentation(workspace)?),
        ("VALIDATION", validation(workspace)?),
    ];
    Output::derive(workspace, PAGE, |current| {
        let mut page = current.to_owned();
        for (name, body) in &regions {
            page = splice(
                &page,
                &format!("<!-- COVERAGE:{name}:BEGIN -->"),
                &format!("<!-- COVERAGE:{name}:END -->"),
                body,
            )?;
        }
        Ok(page)
    })
}

fn authored(workspace: &Workspace) -> Result<String, String> {
    let text = std::fs::read_to_string(workspace.root.join(AUTHORED))
        .map_err(|error| format!("{AUTHORED}: {error}"))?;
    let report: serde_json::Value =
        serde_json::from_str(&text).map_err(|error| format!("{AUTHORED}: {error}"))?;
    let number = |key: &str| report[key].as_u64().unwrap_or(0);
    let list = |key: &str| -> Vec<String> {
        report[key]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|v| v.as_str().map(str::to_owned))
            .collect()
    };
    let (concrete, authored) = (number("concrete"), number("authored"));
    let percent = if concrete == 0 {
        0.0
    } else {
        100.0 * authored as f64 / concrete as f64
    };
    let mut out = vec![
        "| Measure | Entities |".to_owned(),
        "| --- | ---: |".to_owned(),
        format!(
            "| Concrete entities in {} | {concrete} |",
            report["schema"]
                .as_str()
                .unwrap_or("the schema")
                .trim_end_matches(".exp")
                .replace('_', " ")
        ),
        format!("| Created by a writer in the test suite | {authored} ({percent:.1}%) |"),
        format!("| Landed in a model by any path | {} |", number("landed")),
        format!(
            "| Seen only through a codec or fixture, never written | {} |",
            list("seen_but_unwritable").len()
        ),
        format!("| Never produced at all | {} |", list("unproven").len()),
    ];
    for (key, title) in [
        ("unproven", "Entities no test produces"),
        ("seen_but_unwritable", "Entities read but never written"),
    ] {
        let names = list(key);
        if !names.is_empty() {
            out.push(String::new());
            out.push(format!(
                "<details><summary>{title} ({})</summary>\n",
                names.len()
            ));
            out.push(
                names
                    .iter()
                    .map(|n| format!("`{n}`"))
                    .collect::<Vec<_>>()
                    .join(", "),
            );
            out.push("\n</details>".to_owned());
        }
    }
    Ok(out.join("\n"))
}

/// Rows of a tab-separated manifest, keyed by its header.
fn tsv(workspace: &Workspace, rel: &str) -> Result<Vec<BTreeMap<String, String>>, String> {
    let text = std::fs::read_to_string(workspace.root.join(rel))
        .map_err(|error| format!("{rel}: {error}"))?;
    let mut lines = text.lines();
    let header: Vec<&str> = lines
        .next()
        .ok_or_else(|| format!("{rel}: empty"))?
        .split('\t')
        .collect();
    lines
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let cells: Vec<&str> = line.split('\t').collect();
            if cells.len() != header.len() {
                return Err(format!(
                    "{rel}: `{line}` has {} cells, header has {}",
                    cells.len(),
                    header.len()
                ));
            }
            Ok(header
                .iter()
                .map(|h| (*h).to_owned())
                .zip(cells.iter().map(|c| (*c).to_owned()))
                .collect())
        })
        .collect()
}

/// A count table: one row per key, sorted, plus a total.
fn counts(title: &str, rows: impl IntoIterator<Item = String>) -> String {
    let mut tally: BTreeMap<String, usize> = BTreeMap::new();
    for key in rows {
        *tally.entry(key).or_default() += 1;
    }
    let total: usize = tally.values().sum();
    let mut out = vec![format!("| {title} | Count |"), "| --- | ---: |".to_owned()];
    out.extend(
        tally
            .iter()
            .map(|(key, count)| format!("| `{key}` | {count} |")),
    );
    out.push(format!("| **Total** | **{total}** |"));
    out.join("\n")
}

fn geometry(workspace: &Workspace) -> Result<String, String> {
    let support = tsv(workspace, SUPPORT)?;
    let rules = tsv(workspace, WHERE_RULES)?;
    Ok(format!(
        "Every geometry declaration of IFC4 ADD2 TC1, by how this repository handles it \
         (`{SUPPORT}`):\n\n{}\n\nGeometry WHERE rules (`{WHERE_RULES}`):\n\n{}",
        counts("Status", support.iter().map(|r| r["status"].clone())),
        counts("State", rules.iter().map(|r| r["state"].clone())),
    ))
}

fn dispositions(workspace: &Workspace) -> Result<String, String> {
    let rows = tsv(workspace, DISPOSITIONS)?;
    let mut out = vec![
        counts("Disposition", rows.iter().map(|r| r["disposition"].clone())),
        String::new(),
        format!(
            "<details><summary>Every representation item ({})</summary>\n",
            rows.len()
        ),
        "| Entity | Disposition | Owner | Why |".to_owned(),
        "| --- | --- | --- | --- |".to_owned(),
    ];
    for row in &rows {
        out.push(format!(
            "| `{}` | `{}` | `{}` | {} |",
            row["entity"],
            row["disposition"],
            row["owner"],
            row["rationale"].replace('|', "\\|")
        ));
    }
    out.push("\n</details>".to_owned());
    Ok(out.join("\n"))
}

fn presentation(workspace: &Workspace) -> Result<String, String> {
    let consts = Consts::read(workspace, APPEARANCE)?;
    let mut rows = Vec::new();
    for record in consts.records("APPEARANCE_DECLARATIONS")? {
        let name = record
            .field("name")
            .and_then(|e| consts.text(e))
            .ok_or_else(|| consts.shape("APPEARANCE_DECLARATIONS", "a `name` per record"))?;
        let variant = |field: &str| {
            record
                .field(field)
                .and_then(rust_source::variant)
                .map(|(v, _)| v)
                .ok_or_else(|| {
                    consts.shape("APPEARANCE_DECLARATIONS", "`kind` and `support` per record")
                })
        };
        rows.push((name, variant("kind")?, variant("support")?));
    }
    let mut out = vec![
        counts(
            "Support",
            rows.iter().map(|(_, _, support)| support.clone()),
        ),
        String::new(),
        format!(
            "<details><summary>Every appearance declaration ({})</summary>\n",
            rows.len()
        ),
        "| Declaration | Kind | Support |".to_owned(),
        "| --- | --- | --- |".to_owned(),
    ];
    out.extend(
        rows.iter()
            .map(|(name, kind, support)| format!("| `{name}` | {kind} | `{support}` |")),
    );
    out.push("\n</details>".to_owned());
    Ok(out.join("\n"))
}

fn validation(workspace: &Workspace) -> Result<String, String> {
    let consts = Consts::read(workspace, VALIDATION)?;
    let mut out = vec![
        "| Rule | Constrains | Evaluated | Why not |".to_owned(),
        "| --- | --- | --- | --- |".to_owned(),
    ];
    let (mut evaluated, mut total) = (0, 0);
    for record in consts.records("RULES")? {
        let id = record
            .field("id")
            .and_then(|e| consts.text(e))
            .ok_or_else(|| consts.shape("RULES", "an `id` per rule"))?;
        let entity = record
            .field("entity")
            .and_then(rust_source::option)
            .and_then(|e| consts.text(e))
            .map_or_else(|| "(global)".to_owned(), |e| format!("`{e}`"));
        let (support, reason) = record
            .field("support")
            .and_then(rust_source::variant)
            .ok_or_else(|| consts.shape("RULES", "a `support` per rule"))?;
        let implemented = support == "Implemented";
        total += 1;
        if implemented {
            evaluated += 1;
        }
        let reason = reason.and_then(|r| consts.text(r)).unwrap_or_default();
        out.push(format!(
            "| `{id}` | {entity} | {} | {reason} |",
            if implemented { "yes" } else { "no" }
        ));
    }
    Ok(format!(
        "{evaluated} of {total} registered rules are evaluated; the rest are reported as \
         unsupported rather than silently passed (`{VALIDATION}`).\n\n{}",
        out.join("\n")
    ))
}
