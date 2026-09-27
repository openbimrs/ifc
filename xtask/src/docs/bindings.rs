//! The install table and the three bindings' API references.
//!
//! Each reference is read from the binding's own declaration of its surface:
//! the `#[wasm_bindgen]` exports, the public Python module, and the
//! cbindgen-generated C header (itself staleness-tested by
//! `crates/openbim-ifc-capi/tests/header.rs`). A new export reaches the docs with no
//! edit to any page.
//!
//! ## Internal split
//!
//! - `js.rs`: the JavaScript/TypeScript surface of `openbim-ifc-wasm`.
//! - `python.rs`: the `openbim_ifc` Python package, read with Python's `ast`.
//! - `c.rs`: `crates/openbim-ifc-capi/include/openbim_ifc.h`.

mod c;
mod js;
mod python;

use super::release;
use super::Output;
use crate::text::splice;
use crate::workspace::Workspace;

/// `(page, region, generator)` for every binding page region.
pub(super) fn generate(workspace: &Workspace) -> Result<Vec<Output>, String> {
    let install = install_table(workspace)?;
    let js = js::reference(workspace)?;
    let python = python::reference(workspace)?;
    let c = c::reference(workspace)?;
    [
        ("docs/guide/install.md", "INSTALL:TABLE", install),
        ("docs/bindings/javascript.md", "API:JS", js),
        ("docs/bindings/python.md", "API:PYTHON", python),
        ("docs/bindings/c.md", "API:C", c),
    ]
    .into_iter()
    .map(|(page, region, body)| {
        Output::derive(workspace, page, |current| {
            splice(
                current,
                &format!("<!-- {region}:BEGIN -->"),
                &format!("<!-- {region}:END -->"),
                &body,
            )
        })
    })
    .collect()
}

/// One row per installable package: the facade and each language binding.
fn install_table(workspace: &Workspace) -> Result<String, String> {
    let crates = workspace.crates()?;
    let mut rows = vec![
        "| Language | Package | Latest release | Install | Requires | Reference |".to_owned(),
        "| --- | --- | --- | --- | --- | --- |".to_owned(),
    ];
    for (name, language) in [
        ("openbim-ifc", "Rust"),
        ("openbim-ifc-wasm", "JavaScript / TypeScript"),
        ("openbim-ifc-py", "Python"),
        ("openbim-ifc-capi", "C / C++"),
    ] {
        let krate = crates
            .iter()
            .find(|c| c.name == name)
            .ok_or_else(|| format!("install table: `{name}` is not in the workspace"))?;
        let latest = release::latest(workspace, krate)?;
        let registry = release::registries(workspace, krate)
            .into_iter()
            .find(|r| r.kind != "crates.io" || krate.name == "openbim-ifc");
        let (package, command) = match &registry {
            Some(r) if r.kind == "crates.io" => (
                format!("[`{}`]({})", r.package, r.url),
                format!("`cargo add {}`", r.package),
            ),
            Some(r) if r.kind == "npm" => (
                format!("[`{}`]({})", r.package, r.url),
                format!("`npm install {}`", r.package),
            ),
            Some(r) => (
                format!("[`{}`]({})", r.package, r.url),
                format!("`pip install {}`", r.package),
            ),
            None => (
                format!("`{name}` (not published)"),
                "build from source".to_owned(),
            ),
        };
        let released = latest.map_or_else(
            || "not released".to_owned(),
            |r| format!("{} ({})", r.version, r.date),
        );
        let requires = requirement(workspace, krate)?;
        rows.push(format!(
            "| {language} | {package} | {released} | {command} | {requires} | [`{name}`](/reference/crates/{name}) |"
        ));
    }
    Ok(rows.join("\n"))
}

/// The toolchain floor each package declares for itself.
fn requirement(workspace: &Workspace, krate: &crate::workspace::Crate) -> Result<String, String> {
    let dir = workspace.root.join(&krate.dir);
    let read = |rel: &str| std::fs::read_to_string(dir.join(rel)).ok();
    Ok(match krate.name.as_str() {
        "openbim-ifc-wasm" => {
            let manifest: serde_json::Value = read("npm/package.json")
                .and_then(|text| serde_json::from_str(&text).ok())
                .ok_or("crates/openbim-ifc-wasm/npm/package.json is unreadable")?;
            let node = manifest["engines"]["node"]
                .as_str()
                .ok_or("crates/openbim-ifc-wasm/npm/package.json declares no `engines.node`")?;
            format!("Node `{node}`")
        }
        "openbim-ifc-py" => {
            let floor = read("pyproject.toml")
                .and_then(|text| {
                    text.lines().find_map(|line| {
                        let value = line.trim().strip_prefix("requires-python")?;
                        Some(
                            value
                                .trim_start()
                                .strip_prefix('=')?
                                .trim()
                                .trim_matches('"')
                                .to_owned(),
                        )
                    })
                })
                .ok_or("crates/openbim-ifc-py/pyproject.toml declares no `requires-python`")?;
            format!("Python `{floor}`")
        }
        // Checked by crates/openbim-ifc-capi/scripts/check-c.sh, which builds the
        // smoke test with `-std=c11` and `-std=c++17`.
        "openbim-ifc-capi" => "a C11 or C++17 compiler, and Rust to build".to_owned(),
        _ => match &krate.rust_version {
            Some(version) => format!("Rust `{version}`"),
            None => return Err(format!("{}: set `rust-version`", krate.name)),
        },
    })
}

/// First paragraph of a doc text, on one line, safe inside a table cell.
pub(super) fn summary(doc: &str) -> String {
    doc.trim()
        .split("\n\n")
        .next()
        .unwrap_or("")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .replace('|', "\\|")
}
