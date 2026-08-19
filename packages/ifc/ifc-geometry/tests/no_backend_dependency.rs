//! Fast local smoke gates for the IFC/geometry swap boundary.
//!
//! IFC adapters may depend on format-neutral representation crates,
//! but CPU/GPU execution and adapter crates are application choices.
//! `ifc-model/tests/package_architecture.rs` is the authoritative,
//! Cargo-metadata-backed boundary gate.

use std::{collections::BTreeSet, path::PathBuf};

use syn::visit::{self, Visit};

fn ifc_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../packages/ifc")
}

fn uncommented(manifest: &str) -> String {
    manifest
        .lines()
        .map(|line| line.split('#').next().unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n")
}

const LEGACY_REQUEST_NAMES: &[&str] = &["BooleanOp", "CsgShape", "Primitive", "Profile"];

#[derive(Default)]
struct LegacyRequestVisitor {
    root_aliases: BTreeSet<String>,
    violations: Vec<String>,
}

fn legacy_request_violations(source: &str) -> Vec<String> {
    let syntax = syn::parse_file(source).expect("lowering source must parse");
    let mut visitor = LegacyRequestVisitor::default();
    visitor.collect_root_aliases(&syntax);
    visitor.visit_file(&syntax);
    visitor.violations
}

fn is_root_qualifier(name: &str) -> bool {
    matches!(name, "crate" | "self" | "super")
}

fn is_legacy_name(name: &str) -> bool {
    name == "kernel" || LEGACY_REQUEST_NAMES.contains(&name)
}

impl LegacyRequestVisitor {
    fn collect_root_aliases(&mut self, file: &syn::File) {
        loop {
            let before = self.root_aliases.len();
            self.collect_root_aliases_once(file);
            if self.root_aliases.len() == before {
                break;
            }
        }
    }

    fn collect_root_aliases_once(&mut self, file: &syn::File) {
        for item in &file.items {
            match item {
                syn::Item::Use(item) => {
                    collect_use_aliases(&item.tree, false, &mut self.root_aliases)
                }
                syn::Item::ExternCrate(item) if item.ident == "self" => {
                    if let Some((_, alias)) = &item.rename {
                        self.root_aliases.insert(alias.to_string());
                    }
                }
                syn::Item::Mod(item) => {
                    if let Some((_, items)) = &item.content {
                        let nested = syn::File {
                            shebang: None,
                            attrs: Vec::new(),
                            items: items.clone(),
                        };
                        self.collect_root_aliases(&nested);
                    }
                }
                _ => {}
            }
        }
    }
}

fn collect_use_aliases(tree: &syn::UseTree, rooted: bool, aliases: &mut BTreeSet<String>) {
    match tree {
        syn::UseTree::Path(path) => {
            let name = path.ident.to_string();
            let rooted = rooted || is_root_qualifier(&name) || aliases.contains(&name);
            collect_use_aliases(&path.tree, rooted, aliases);
        }
        syn::UseTree::Rename(rename)
            if is_root_qualifier(&rename.ident.to_string())
                || aliases.contains(&rename.ident.to_string()) =>
        {
            aliases.insert(rename.rename.to_string());
        }
        syn::UseTree::Group(group) => {
            for item in &group.items {
                collect_use_aliases(item, rooted, aliases);
            }
        }
        _ => {}
    }
}

fn inspect_use_tree(
    tree: &syn::UseTree,
    rooted: bool,
    root_aliases: &BTreeSet<String>,
    violations: &mut Vec<String>,
) {
    match tree {
        syn::UseTree::Path(path) => {
            let name = path.ident.to_string();
            let rooted = rooted || is_root_qualifier(&name) || root_aliases.contains(&name);
            if rooted && is_legacy_name(&path.ident.to_string()) {
                violations.push(format!("legacy import segment `{}`", path.ident));
            }
            let tail_rooted =
                rooted && !(name == "super" && matches!(path.tree.as_ref(), syn::UseTree::Glob(_)));
            inspect_use_tree(&path.tree, tail_rooted, root_aliases, violations);
        }
        syn::UseTree::Name(name) if rooted && is_legacy_name(&name.ident.to_string()) => {
            violations.push(format!("legacy import `{}`", name.ident));
        }
        syn::UseTree::Rename(name) if rooted && is_legacy_name(&name.ident.to_string()) => {
            violations.push(format!("legacy renamed import `{}`", name.ident));
        }
        syn::UseTree::Glob(_) if rooted => violations.push("crate-root glob import".to_owned()),
        syn::UseTree::Group(group) => {
            for item in &group.items {
                inspect_use_tree(item, rooted, root_aliases, violations);
            }
        }
        _ => {}
    }
}

fn flatten_macro_tokens(tokens: proc_macro2::TokenStream, output: &mut String) {
    for token in tokens {
        match token {
            proc_macro2::TokenTree::Ident(ident) => output.push_str(&ident.to_string()),
            proc_macro2::TokenTree::Punct(punct) => output.push(punct.as_char()),
            proc_macro2::TokenTree::Group(group) => flatten_macro_tokens(group.stream(), output),
            proc_macro2::TokenTree::Literal(_) => {}
        }
    }
}

impl<'ast> Visit<'ast> for LegacyRequestVisitor {
    fn visit_item_use(&mut self, item: &'ast syn::ItemUse) {
        inspect_use_tree(&item.tree, false, &self.root_aliases, &mut self.violations);
        visit::visit_item_use(self, item);
    }

    fn visit_path(&mut self, path: &'ast syn::Path) {
        let Some(first) = path.segments.first() else {
            return;
        };
        let first = first.ident.to_string();
        let rooted = is_root_qualifier(&first) || self.root_aliases.contains(&first);
        if rooted {
            for segment in &path.segments {
                let name = segment.ident.to_string();
                if is_legacy_name(&name) {
                    self.violations
                        .push(format!("legacy path segment `{name}`"));
                    break;
                }
            }
        }
        visit::visit_path(self, path);
    }

    fn visit_macro(&mut self, item: &'ast syn::Macro) {
        let mut tokens = String::new();
        flatten_macro_tokens(item.tokens.clone(), &mut tokens);
        let roots = ["crate", "super", "self"]
            .into_iter()
            .chain(self.root_aliases.iter().map(String::as_str));
        for root in roots {
            for legacy in LEGACY_REQUEST_NAMES.iter().copied().chain(["kernel"]) {
                if tokens.contains(&format!("{root}::{legacy}")) {
                    self.violations
                        .push(format!("legacy macro path `{root}::{legacy}`"));
                }
            }
        }
        visit::visit_macro(self, item);
    }
}

#[test]
fn ifc_crates_never_depend_on_geometry_execution_crates() {
    let mut checked = 0;
    for entry in std::fs::read_dir(ifc_dir()).expect("packages/ifc must exist") {
        let manifest = entry.expect("entry").path().join("Cargo.toml");
        if !manifest.exists() {
            continue;
        }
        let body = uncommented(&std::fs::read_to_string(&manifest).expect("manifest readable"));
        assert!(
            !body.contains("geom-backend-") && !body.contains("geom-kernel"),
            "{} binds IFC semantics to a geometry contract/execution crate. Emit neutral geom-model values and select operation providers in an app crate.",
            manifest.display()
        );
        checked += 1;
    }
    assert!(
        checked >= 7,
        "expected every packages/ifc crate, saw {checked}"
    );
}

#[test]
fn active_lowering_does_not_use_legacy_request_vocabulary() {
    fn collect_rs(dir: &std::path::Path, files: &mut Vec<PathBuf>) {
        for entry in std::fs::read_dir(dir).expect("read lower directory") {
            let path = entry.expect("lower entry").path();
            if path.is_dir() {
                collect_rs(&path, files);
            } else if path.extension().and_then(|extension| extension.to_str()) == Some("rs") {
                files.push(path);
            }
        }
    }

    let lower = ifc_dir().join("ifc-geometry/src/lower");
    let mut files = Vec::new();
    collect_rs(&lower, &mut files);
    assert!(!files.is_empty(), "expected active lowering modules");

    for file in files {
        let source = std::fs::read_to_string(&file).expect("lower module readable");
        let violations = legacy_request_violations(&source);
        assert!(
            violations.is_empty(),
            "{} reaches legacy pre-DAG vocabulary from active lowering: {}",
            file.display(),
            violations.join(", ")
        );
    }
}

/// Geometry access is an explicit allowlist, not an accident.
const MAY_USE_GEOMETRY: &[&str] = &["ifc-geometry", "ifc-alignment", "ifc-georef"];

#[test]
fn geometry_access_is_limited_to_the_allowlist() {
    for entry in std::fs::read_dir(ifc_dir()).expect("ifc dir must exist") {
        let path = entry.expect("entry").path();
        let name = path
            .file_name()
            .expect("name")
            .to_string_lossy()
            .to_string();
        let manifest = path.join("Cargo.toml");
        if !manifest.exists() || MAY_USE_GEOMETRY.contains(&name.as_str()) {
            continue;
        }
        let body = uncommented(&std::fs::read_to_string(&manifest).expect("manifest readable"));
        assert!(
            !body.contains("geom-"),
            "packages/ifc/{name} depends on geometry but is not allowlisted. Property, cost, \
             quantity, and classification consumers must not compile geometry accidentally."
        );
    }
}

#[test]
fn allowlist_names_only_real_crates() {
    for allowed in MAY_USE_GEOMETRY {
        assert!(
            ifc_dir().join(allowed).join("Cargo.toml").exists(),
            "MAY_USE_GEOMETRY names missing crate `{allowed}`"
        );
    }
}

#[test]
fn lowering_gate_resolves_aliases_globs_and_spaced_paths() {
    let forbidden = [
        "type Probe = super::kernel::Primitive;",
        "use crate as adapter; type Probe = adapter::Primitive;",
        "type Probe = crate :: kernel :: Primitive;",
        "use crate::*; type Probe = Primitive;",
        "macro_rules! legacy { () => { crate::kernel::Primitive } } type Probe = legacy!();",
        "extern crate self as adapter; type Probe = adapter::Profile;",
        "use adapter as bridge; use crate as adapter; type Probe = bridge::Primitive;",
    ];
    for source in forbidden {
        assert!(
            !legacy_request_violations(source).is_empty(),
            "missed: {source}"
        );
    }
}

#[test]
fn lowering_gate_allows_neutral_types_and_legacy_words_in_data() {
    let source = r#"use geom_profile::Profile;
const NOTE: &str = "crate::kernel::Primitive";
// crate::Profile is documentation only.
"#;
    assert!(legacy_request_violations(source).is_empty());
}
