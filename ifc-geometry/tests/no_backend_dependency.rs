//! Fast local smoke gates for the IFC/geometry swap boundary.
//!
//! IFC adapters may depend on format-neutral representation crates,
//! but CPU/GPU execution and adapter crates are application choices.
//! `ifc-model/tests/package_architecture.rs` is the authoritative,
//! Cargo-metadata-backed boundary gate.

use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

use proc_macro2::{TokenStream, TokenTree};
use syn::visit::{self, Visit};

/// `packages/ifc/` — the group directory holding the whole IFC crate family.
fn ifc_group_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

/// The IFC layer: every `ifc-*` crate plus the `openbim-ifc` facade. Selecting
/// by name as well as by directory keeps this boundary test meaningful -- a
/// directory walk would otherwise sweep in the openBIM standard crates, which
/// have their own, different rules.
fn ifc_layer_crates() -> Vec<PathBuf> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(ifc_group_dir()).expect("packages/ must exist") {
        let path = entry.expect("entry").path();
        let name = path
            .file_name()
            .expect("name")
            .to_string_lossy()
            .to_string();
        if !(name.starts_with("ifc-") || name == "openbim-ifc") {
            continue;
        }
        if path.join("Cargo.toml").exists() {
            out.push(path);
        }
    }
    out
}

fn uncommented(manifest: &str) -> String {
    manifest
        .lines()
        .map(|line| line.split('#').next().unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n")
}

const LEGACY_REQUEST_NAMES: &[&str] = &["BooleanOp", "CsgShape", "Primitive", "Profile"];

fn is_root_qualifier(name: &str) -> bool {
    matches!(normalize_ident(name), "crate" | "self" | "super")
}

/// Raw-identifier syntax (`r#kernel`) must not defeat name comparisons:
/// normalize by stripping the `r#` prefix before matching.
fn normalize_ident(name: &str) -> &str {
    name.strip_prefix("r#").unwrap_or(name)
}

fn is_legacy_name(name: &str) -> bool {
    let name = normalize_ident(name);
    name == "kernel" || LEGACY_REQUEST_NAMES.contains(&name)
}

/// One matcher/transcriber arm of a locally defined `macro_rules!` macro,
/// restricted to the simple "list of `$name:frag` params, path-shaped body"
/// subset needed to trace where an invocation's arguments land. Anything
/// richer (nested groups, literals, repetition) is left unregistered and
/// falls back to the raw-token substring scan below.
struct MacroTemplate {
    params: Vec<String>,
    segments: Vec<Seg>,
}

enum Seg {
    Literal(String),
    Var(usize),
}

/// Splits a macro invocation's argument tokens on top-level commas.
fn split_top_level(tokens: TokenStream) -> Vec<TokenStream> {
    let mut groups = Vec::new();
    let mut current = Vec::new();
    for token in tokens {
        if let TokenTree::Punct(punct) = &token {
            if punct.as_char() == ',' {
                groups.push(TokenStream::from_iter(std::mem::take(&mut current)));
                continue;
            }
        }
        current.push(token);
    }
    if !current.is_empty() {
        groups.push(TokenStream::from_iter(current));
    }
    groups
}

fn flatten_macro_tokens(tokens: TokenStream, output: &mut String) {
    for token in tokens {
        match token {
            TokenTree::Ident(ident) => output.push_str(normalize_ident(&ident.to_string())),
            TokenTree::Punct(punct) => output.push(punct.as_char()),
            TokenTree::Group(group) => flatten_macro_tokens(group.stream(), output),
            TokenTree::Literal(_) => {}
        }
    }
}

fn flatten_to_string(tokens: TokenStream) -> String {
    let mut out = String::new();
    flatten_macro_tokens(tokens, &mut out);
    out
}

/// Parses a `$name:fragment` matcher list (the only matcher shape this gate
/// understands). Returns `None` for anything richer (literals, nested
/// groups, repetition) so the caller leaves that macro unregistered.
fn parse_matcher_params(tokens: TokenStream) -> Option<Vec<String>> {
    let mut params = Vec::new();
    let mut iter = tokens.into_iter().peekable();
    while let Some(token) = iter.next() {
        match token {
            TokenTree::Punct(dollar) if dollar.as_char() == '$' => {
                let TokenTree::Ident(name) = iter.next()? else {
                    return None;
                };
                match iter.next()? {
                    TokenTree::Punct(colon) if colon.as_char() == ':' => {}
                    _ => return None,
                }
                let TokenTree::Ident(_fragment_kind) = iter.next()? else {
                    return None;
                };
                params.push(normalize_ident(&name.to_string()).to_owned());
            }
            TokenTree::Punct(comma) if comma.as_char() == ',' => {}
            _ => return None,
        }
    }
    Some(params)
}

/// Parses a path-shaped transcriber body (`$root :: kernel :: Primitive`)
/// into literal/variable segments. Returns `None` for anything outside
/// that shape.
fn parse_body_segments(tokens: TokenStream, params: &[String]) -> Option<Vec<Seg>> {
    let mut segments = Vec::new();
    let mut iter = tokens.into_iter().peekable();
    while let Some(token) = iter.next() {
        match token {
            TokenTree::Punct(dollar) if dollar.as_char() == '$' => {
                let TokenTree::Ident(name) = iter.next()? else {
                    return None;
                };
                let name = normalize_ident(&name.to_string()).to_owned();
                let index = params.iter().position(|param| *param == name)?;
                segments.push(Seg::Var(index));
            }
            TokenTree::Ident(ident) => {
                segments.push(Seg::Literal(normalize_ident(&ident.to_string()).to_owned()));
            }
            TokenTree::Punct(colon) if colon.as_char() == ':' => {}
            _ => return None,
        }
    }
    Some(segments)
}

/// Splits `macro_rules! name { (m1) => { b1 }; (m2) => { b2 }; ... }`
/// tokens into its arms and registers the first one this gate can parse.
fn parse_macro_rules(tokens: TokenStream) -> Option<MacroTemplate> {
    let mut iter = tokens.into_iter().peekable();
    let matcher = match iter.next()? {
        TokenTree::Group(group) => group.stream(),
        _ => return None,
    };
    // Skip the `=>` fat arrow, tokenized as two adjacent `Punct`s.
    iter.next()?;
    iter.next()?;
    let body = match iter.next()? {
        TokenTree::Group(group) => group.stream(),
        _ => return None,
    };
    let params = parse_matcher_params(matcher)?;
    let segments = parse_body_segments(body, &params)?;
    Some(MacroTemplate { params, segments })
}

fn substitute_segments(template: &MacroTemplate, args: &[String]) -> Option<Vec<String>> {
    if args.len() != template.params.len() {
        return None;
    }
    template
        .segments
        .iter()
        .map(|segment| match segment {
            Seg::Literal(literal) => Some(literal.clone()),
            Seg::Var(index) => args.get(*index).cloned(),
        })
        .collect()
}

fn check_segments(segments: &[String], root_aliases: &BTreeSet<String>) -> Option<String> {
    let first = segments.first()?;
    let rooted = is_root_qualifier(first) || root_aliases.contains(first);
    if !rooted {
        return None;
    }
    segments
        .iter()
        .find(|segment| is_legacy_name(segment))
        .map(|hit| format!("legacy macro-expanded path segment `{hit}`"))
}

/// Collects root aliases (`use crate as X`, `extern crate self as X`, plus
/// transitive aliases-of-aliases) and locally defined `macro_rules!`
/// templates anywhere in the file -- including inside function bodies,
/// impls, and nested modules, since `syn`'s default `Visit` recursion walks
/// every scope unless a method here short-circuits it.
struct CollectorVisitor<'a> {
    root_aliases: &'a mut BTreeSet<String>,
    macros: &'a mut BTreeMap<String, MacroTemplate>,
}

impl<'ast> Visit<'ast> for CollectorVisitor<'_> {
    fn visit_item_use(&mut self, item: &'ast syn::ItemUse) {
        collect_use_aliases(&item.tree, false, self.root_aliases);
        visit::visit_item_use(self, item);
    }

    fn visit_item_extern_crate(&mut self, item: &'ast syn::ItemExternCrate) {
        if item.ident == "self" {
            if let Some((_, alias)) = &item.rename {
                self.root_aliases.insert(alias.to_string());
            }
        }
        visit::visit_item_extern_crate(self, item);
    }

    fn visit_item_macro(&mut self, item: &'ast syn::ItemMacro) {
        if item.mac.path.is_ident("macro_rules") {
            if let Some(name) = &item.ident {
                if let Some(template) = parse_macro_rules(item.mac.tokens.clone()) {
                    self.macros.insert(name.to_string(), template);
                }
            }
        }
        visit::visit_item_macro(self, item);
    }
}

fn collect_context(file: &syn::File) -> (BTreeSet<String>, BTreeMap<String, MacroTemplate>) {
    let mut root_aliases = BTreeSet::new();
    let mut macros = BTreeMap::new();
    // Aliases can chain (`use crate as a; use a as b;`) and, unlike real
    // Rust name resolution, this single-pass visitor is order-sensitive;
    // re-running to a fixpoint makes multi-hop chains resolve regardless
    // of declaration order.
    loop {
        let before = root_aliases.len();
        let mut visitor = CollectorVisitor {
            root_aliases: &mut root_aliases,
            macros: &mut macros,
        };
        visitor.visit_file(file);
        if root_aliases.len() == before {
            break;
        }
    }
    (root_aliases, macros)
}

fn collect_use_aliases(tree: &syn::UseTree, rooted: bool, aliases: &mut BTreeSet<String>) {
    match tree {
        syn::UseTree::Path(path) => {
            let name = normalize_ident(&path.ident.to_string()).to_owned();
            let rooted = rooted || is_root_qualifier(&name) || aliases.contains(&name);
            collect_use_aliases(&path.tree, rooted, aliases);
        }
        syn::UseTree::Rename(rename) => {
            let name = normalize_ident(&rename.ident.to_string()).to_owned();
            if is_root_qualifier(&name) || aliases.contains(&name) {
                aliases.insert(rename.rename.to_string());
            }
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
            let name = normalize_ident(&path.ident.to_string()).to_owned();
            let rooted = rooted || is_root_qualifier(&name) || root_aliases.contains(&name);
            if rooted && is_legacy_name(&name) {
                violations.push(format!("legacy import segment `{}`", path.ident));
            }
            inspect_use_tree(&path.tree, rooted, root_aliases, violations);
        }
        syn::UseTree::Name(name) if rooted && is_legacy_name(&name.ident.to_string()) => {
            violations.push(format!("legacy import `{}`", name.ident));
        }
        syn::UseTree::Rename(name) if rooted && is_legacy_name(&name.ident.to_string()) => {
            violations.push(format!("legacy renamed import `{}`", name.ident));
        }
        // `super::*`/`crate::*`/aliased globs are not flagged standalone --
        // `use super::*;` inside a test module is a normal, safe Rust
        // idiom. `LegacyRequestVisitor::has_rooted_glob` tracks their
        // presence separately so a later *bare* reference to a legacy name
        // can be recognized as glob-exposed.
        syn::UseTree::Glob(_) => {}
        syn::UseTree::Group(group) => {
            for item in &group.items {
                inspect_use_tree(item, rooted, root_aliases, violations);
            }
        }
        _ => {}
    }
}

fn contains_rooted_glob(
    tree: &syn::UseTree,
    rooted: bool,
    root_aliases: &BTreeSet<String>,
) -> bool {
    match tree {
        syn::UseTree::Path(path) => {
            let name = normalize_ident(&path.ident.to_string()).to_owned();
            let rooted = rooted || is_root_qualifier(&name) || root_aliases.contains(&name);
            contains_rooted_glob(&path.tree, rooted, root_aliases)
        }
        syn::UseTree::Group(group) => group
            .items
            .iter()
            .any(|item| contains_rooted_glob(item, rooted, root_aliases)),
        syn::UseTree::Glob(_) => rooted,
        _ => false,
    }
}

struct GlobDetector<'a> {
    root_aliases: &'a BTreeSet<String>,
    found: bool,
}

impl<'ast> Visit<'ast> for GlobDetector<'_> {
    fn visit_item_use(&mut self, item: &'ast syn::ItemUse) {
        if contains_rooted_glob(&item.tree, false, self.root_aliases) {
            self.found = true;
        }
        visit::visit_item_use(self, item);
    }
}

struct LegacyRequestVisitor {
    root_aliases: BTreeSet<String>,
    macros: BTreeMap<String, MacroTemplate>,
    has_rooted_glob: bool,
    violations: Vec<String>,
}

fn legacy_request_violations(source: &str) -> Vec<String> {
    let syntax = syn::parse_file(source).expect("lowering source must parse");
    let (root_aliases, macros) = collect_context(&syntax);
    let mut glob_detector = GlobDetector {
        root_aliases: &root_aliases,
        found: false,
    };
    glob_detector.visit_file(&syntax);
    let has_rooted_glob = glob_detector.found;
    let mut visitor = LegacyRequestVisitor {
        root_aliases,
        macros,
        has_rooted_glob,
        violations: Vec::new(),
    };
    visitor.visit_file(&syntax);
    visitor.violations
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
        let first_name = normalize_ident(&first.ident.to_string()).to_owned();
        let rooted = is_root_qualifier(&first_name) || self.root_aliases.contains(&first_name);
        if rooted {
            for segment in &path.segments {
                let name = normalize_ident(&segment.ident.to_string()).to_owned();
                if is_legacy_name(&name) {
                    self.violations
                        .push(format!("legacy path segment `{name}`"));
                    break;
                }
            }
        } else if self.has_rooted_glob && is_legacy_name(&first_name) {
            // A bare reference whose first segment is an exact legacy
            // name, with a rooted glob (`use crate::*;`, `use super::*;`,
            // ...) in scope, can be that legacy name re-exposed
            // unqualified by the glob. This is a coarse, deliberately
            // conservative check -- true resolution of what a glob
            // actually brings into scope needs a real name resolver -- but
            // it is narrow enough (exact match against five specific
            // legacy names) to stay silent on ordinary code.
            self.violations.push(format!(
                "bare legacy name `{first_name}` reachable through a rooted glob import"
            ));
        }
        visit::visit_path(self, path);
    }

    fn visit_macro(&mut self, item: &'ast syn::Macro) {
        // Backstop: a raw substring scan over the invocation's own tokens
        // catches legacy vocabulary written directly at the call site
        // (`external_macro!(crate::kernel::Primitive)`), independent of
        // whether the macro itself is one this gate can model.
        let flat = flatten_to_string(item.tokens.clone());
        let roots = ["crate", "super", "self"]
            .into_iter()
            .chain(self.root_aliases.iter().map(String::as_str));
        for root in roots {
            for legacy in LEGACY_REQUEST_NAMES.iter().copied().chain(["kernel"]) {
                if flat.contains(&format!("{root}::{legacy}")) {
                    self.violations
                        .push(format!("legacy macro path `{root}::{legacy}`"));
                }
            }
        }

        // Primary defense: trace arguments through a locally defined
        // `macro_rules!` template so `legacy!(adapter)` is caught even
        // when `adapter` is a `use crate as adapter;` alias substituted
        // into the macro body, not written literally at the call site.
        if let Some(name) = item.path.get_ident() {
            if let Some(template) = self.macros.get(&name.to_string()) {
                let args: Vec<String> = split_top_level(item.tokens.clone())
                    .into_iter()
                    .map(flatten_to_string)
                    .collect();
                if let Some(segments) = substitute_segments(template, &args) {
                    if let Some(violation) = check_segments(&segments, &self.root_aliases) {
                        self.violations.push(violation);
                    }
                }
            }
        }

        visit::visit_macro(self, item);
    }
}

#[test]
fn ifc_crates_never_depend_on_geometry_execution_crates() {
    let mut checked = 0;
    for path in ifc_layer_crates() {
        let manifest = path.join("Cargo.toml");
        let body = uncommented(&std::fs::read_to_string(&manifest).expect("manifest readable"));
        assert!(
            !body.contains("axiolid-backend-") && !body.contains("axiolid-kernel"),
            "{} binds IFC semantics to a geometry contract/execution crate. Emit neutral axiolid-model values and select operation providers in an app crate.",
            manifest.display()
        );
        checked += 1;
    }
    assert!(
        checked >= 7,
        "expected every IFC-layer crate, saw {checked}"
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

    let lower = ifc_group_dir().join("ifc-geometry/src/lower");
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
    for path in ifc_layer_crates() {
        let name = path
            .file_name()
            .expect("name")
            .to_string_lossy()
            .to_string();
        let manifest = path.join("Cargo.toml");
        if MAY_USE_GEOMETRY.contains(&name.as_str()) {
            continue;
        }
        let body = uncommented(&std::fs::read_to_string(&manifest).expect("manifest readable"));
        assert!(
            !body.contains("axiolid-"),
            "packages/{name} depends on geometry but is not allowlisted. Property, cost, \
             quantity, and classification consumers must not compile geometry accidentally."
        );
    }
}

#[test]
fn allowlist_names_only_real_crates() {
    for allowed in MAY_USE_GEOMETRY {
        assert!(
            ifc_group_dir().join(allowed).join("Cargo.toml").exists(),
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
fn lowering_gate_resolves_raw_idents_block_scoped_aliases_and_macro_arguments() {
    let forbidden = [
        // Raw-identifier syntax must not defeat name comparison.
        "type Probe = crate::r#kernel::r#Primitive;",
        // A `use` alias introduced inside a function body, not at module
        // scope, must still be tracked.
        "fn lower() { use crate as adapter; type Probe = adapter::kernel::Primitive; }",
        // `use super::*;` must not be exempted from the rooted-glob check.
        "mod inner { use super::*; type Probe = kernel::Primitive; }",
        // A macro parameter substituted with a root alias must be traced
        // through the macro body to the invocation site.
        "use crate as adapter; macro_rules! legacy { ($root:ident) => { $root::kernel::Primitive } } type Probe = legacy!(adapter);",
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
    let source = r#"use axiolid_profile::Profile;
const NOTE: &str = "crate::kernel::Primitive";
// crate::Profile is documentation only.
"#;
    assert!(legacy_request_violations(source).is_empty());
}
