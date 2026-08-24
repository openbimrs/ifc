//! Every Rust source file must be reachable from a Cargo target root.
//!
//! Rust quietly ignores an undeclared `.rs` file. This gate asks Cargo for every
//! target root and uses Rust syntax, not text matching, to traverse external
//! modules.

use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};

use cargo_metadata::{Metadata, MetadataCommand, Package};
use syn::ext::IdentExt;
use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::{Attribute, Expr, Item, Lit, Meta, Token};

#[path = "support/module_reachability_cases.rs"]
mod module_reachability_cases;

type ModuleContext = (PathBuf, PathBuf);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PublicContract {
    Disabled,
    Empty,
    Present,
}

fn contract_status(present: bool) -> PublicContract {
    if present {
        PublicContract::Present
    } else {
        PublicContract::Empty
    }
}

fn lexical_normalize(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => match normalized.components().next_back() {
                Some(Component::Normal(_)) => {
                    normalized.pop();
                }
                Some(Component::ParentDir) | None if !normalized.has_root() => {
                    normalized.push("..");
                }
                _ => {}
            },
            _ => normalized.push(component.as_os_str()),
        }
    }
    normalized
}

fn normalized_context(base: &Path) -> PathBuf {
    base.canonicalize()
        .unwrap_or_else(|_| lexical_normalize(base))
}

fn module_context(source: &Path, base: &Path) -> ModuleContext {
    (
        source
            .canonicalize()
            .unwrap_or_else(|_| lexical_normalize(source)),
        normalized_context(base),
    )
}

fn metadata() -> Metadata {
    MetadataCommand::new()
        .no_deps()
        .exec()
        .expect("cargo metadata must describe the runtime workspace")
}

/// The IFC-layer packages: every `ifc-*` crate plus the `openbim-ifc` facade.
///
/// `packages/` is flat, so membership is decided by crate NAME rather than by
/// parent directory. A directory-shaped filter would silently sweep in the
/// openBIM standard crates -- and, worse, a filter that matched nothing would
/// make this whole test vacuously pass. The `>= 18` assertion at the call site
/// is the guard against exactly that.
fn ifc_packages() -> Vec<Package> {
    let metadata = metadata();
    let ifc_root = metadata.workspace_root.as_std_path().join("packages/ifc");
    metadata
        .packages
        .into_iter()
        .filter(|package| {
            let is_ifc_layer = package.name.starts_with("ifc-") || package.name == "openbim-ifc";
            let under_ifc_group = package
                .manifest_path
                .as_std_path()
                .parent()
                .and_then(Path::parent)
                == Some(ifc_root.as_path());
            is_ifc_layer && under_ifc_group
        })
        .collect()
}

fn rust_files(dir: &Path, out: &mut BTreeSet<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if !matches!(
                path.file_name().and_then(|name| name.to_str()),
                Some("target" | ".git")
            ) {
                rust_files(&path, out);
            }
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            out.insert(path.canonicalize().unwrap_or(path));
        }
    }
}

fn cfg_value(meta: &Meta) -> Option<bool> {
    let Meta::List(list) = meta else {
        return None;
    };
    let values: Vec<_> = Punctuated::<Meta, Token![,]>::parse_terminated
        .parse2(list.tokens.clone())
        .ok()?
        .iter()
        .map(cfg_value)
        .collect();
    if list.path.is_ident("all") {
        if values.contains(&Some(false)) {
            Some(false)
        } else if values.iter().all(Option::is_some) {
            Some(true)
        } else {
            None
        }
    } else if list.path.is_ident("any") {
        if values.contains(&Some(true)) {
            Some(true)
        } else if values.iter().all(|value| *value == Some(false)) {
            Some(false)
        } else {
            None
        }
    } else if list.path.is_ident("not") && values.len() == 1 {
        values[0].map(|value| !value)
    } else {
        None
    }
}

fn collect_effective_meta(meta: &Meta, effective: &mut Vec<Meta>) {
    let Meta::List(list) = meta else {
        effective.push(meta.clone());
        return;
    };
    if !list.path.is_ident("cfg_attr") {
        effective.push(meta.clone());
        return;
    }

    let Ok(arguments) = Punctuated::<Meta, Token![,]>::parse_terminated.parse2(list.tokens.clone())
    else {
        return;
    };
    let mut arguments = arguments.into_iter();
    let Some(predicate) = arguments.next() else {
        return;
    };
    if cfg_value(&predicate) == Some(true) {
        for nested in arguments {
            collect_effective_meta(&nested, effective);
        }
    }
}

fn effective_metas(attributes: &[Attribute]) -> Vec<Meta> {
    let mut effective = Vec::new();
    for attribute in attributes {
        collect_effective_meta(&attribute.meta, &mut effective);
    }
    effective
}

fn cfg_attribute_value(meta: &Meta) -> Option<bool> {
    let Meta::List(list) = meta else {
        return None;
    };
    if !list.path.is_ident("cfg") {
        return None;
    }
    syn::parse2::<Meta>(list.tokens.clone())
        .ok()
        .and_then(|predicate| cfg_value(&predicate))
}

fn is_statically_disabled(attributes: &[Attribute]) -> bool {
    effective_metas(attributes)
        .iter()
        .any(|meta| cfg_attribute_value(meta) == Some(false))
}

fn path_override(attributes: &[Attribute]) -> Option<PathBuf> {
    effective_metas(attributes).iter().find_map(|meta| {
        if !meta.path().is_ident("path") {
            return None;
        }
        let Meta::NameValue(value) = meta else {
            return None;
        };
        let Expr::Lit(expression) = &value.value else {
            return None;
        };
        let Lit::Str(path) = &expression.lit else {
            return None;
        };
        Some(PathBuf::from(path.value()))
    })
}

struct ResolvedModule {
    source: PathBuf,
    base: PathBuf,
}

fn external_module_path(
    module: &syn::ItemMod,
    source: &Path,
    base: &Path,
    path_base: &Path,
) -> Result<ResolvedModule, String> {
    let name = module.ident.unraw().to_string();
    if let Some(path) = path_override(&module.attrs) {
        let child = path_base.join(path);
        if !child.is_file() {
            return Err(format!(
                "{} declares {name}, but path override {} does not exist",
                source.display(),
                child.display()
            ));
        }
        let child_base = child
            .parent()
            .expect("path-overridden Rust source must have a parent")
            .to_path_buf();
        return Ok(ResolvedModule {
            source: child,
            base: child_base,
        });
    }

    let child_base = base.join(&name);
    let flat = base.join(format!("{name}.rs"));
    let nested = child_base.join("mod.rs");
    if flat.is_file() {
        Ok(ResolvedModule {
            source: flat,
            base: child_base,
        })
    } else if nested.is_file() {
        Ok(ResolvedModule {
            source: nested,
            base: child_base,
        })
    } else {
        Err(format!(
            "{} declares {name}, but neither {} nor {} exists",
            source.display(),
            flat.display(),
            nested.display()
        ))
    }
}

fn visit_items(
    items: &[Item],
    source: &Path,
    base: &Path,
    path_base: &Path,
    reached: &mut BTreeSet<PathBuf>,
    visited: &mut BTreeSet<ModuleContext>,
    missing: &mut Vec<String>,
) {
    for module in items.iter().filter_map(|item| match item {
        Item::Mod(module) if !is_statically_disabled(&module.attrs) => Some(module),
        _ => None,
    }) {
        let name = module.ident.unraw().to_string();
        if let Some((_, inline_items)) = &module.content {
            let child_base = base.join(name);
            visit_items(
                inline_items,
                source,
                &child_base,
                &child_base,
                reached,
                visited,
                missing,
            );
            continue;
        }

        let child = match external_module_path(module, source, base, path_base) {
            Ok(child) => child,
            Err(error) => {
                missing.push(error);
                continue;
            }
        };
        visit_file_at_base(&child.source, child.base, reached, visited, missing);
    }
}

fn visit_file_at_base(
    source: &Path,
    base: PathBuf,
    reached: &mut BTreeSet<PathBuf>,
    visited: &mut BTreeSet<ModuleContext>,
    missing: &mut Vec<String>,
) {
    let key = module_context(source, &base);
    if !visited.insert(key.clone()) {
        return;
    }
    reached.insert(key.0);
    let text = std::fs::read_to_string(source)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", source.display()));
    let syntax = syn::parse_file(&text)
        .unwrap_or_else(|error| panic!("cannot parse {}: {error}", source.display()));
    if is_statically_disabled(&syntax.attrs) {
        return;
    }
    let path_base = source
        .parent()
        .expect("Rust module source must have a parent");
    visit_items(
        &syntax.items,
        source,
        &base,
        path_base,
        reached,
        visited,
        missing,
    );
}

fn visit_target_root(source: &Path, reached: &mut BTreeSet<PathBuf>, missing: &mut Vec<String>) {
    let base = source
        .parent()
        .expect("Cargo target root must have a parent")
        .to_path_buf();
    visit_file_at_base(source, base, reached, &mut BTreeSet::new(), missing);
}

#[test]
fn every_ifc_source_file_is_reachable_from_a_cargo_target() {
    let packages = ifc_packages();
    assert!(
        packages.len() >= 18,
        "expected all IFC crates, found {}",
        packages.len()
    );

    let mut missing = Vec::new();
    let mut orphaned = Vec::new();
    for package in packages {
        let crate_dir = package
            .manifest_path
            .as_std_path()
            .parent()
            .expect("manifest parent");
        let mut all = BTreeSet::new();
        let mut reached = BTreeSet::new();
        rust_files(crate_dir, &mut all);

        let roots: BTreeSet<_> = package
            .targets
            .iter()
            .map(|target| target.src_path.as_std_path().to_path_buf())
            .filter(|root| root.starts_with(crate_dir))
            .collect();
        assert!(
            !roots.is_empty(),
            "{} exposes no Cargo target roots",
            package.name
        );
        for root in roots {
            visit_target_root(&root, &mut reached, &mut missing);
        }
        for path in all.difference(&reached) {
            orphaned.push(path.display().to_string());
        }
    }

    assert!(
        missing.is_empty(),
        "declared modules with missing source files:\n{}",
        missing.join("\n")
    );
    assert!(
        orphaned.is_empty(),
        "Rust files outside every Cargo target/module tree:\n{}",
        orphaned.join("\n")
    );
}

fn is_public(visibility: &syn::Visibility) -> bool {
    matches!(visibility, syn::Visibility::Public(_))
}

fn use_tree_exports_name(tree: &syn::UseTree) -> bool {
    match tree {
        syn::UseTree::Path(path) => use_tree_exports_name(&path.tree),
        syn::UseTree::Group(group) => group.items.iter().any(use_tree_exports_name),
        syn::UseTree::Name(_) | syn::UseTree::Rename(_) | syn::UseTree::Glob(_) => true,
    }
}

fn is_public_contract_item(item: &Item) -> bool {
    match item {
        Item::Const(item) => is_public(&item.vis) && !is_statically_disabled(&item.attrs),
        Item::Enum(item) => is_public(&item.vis) && !is_statically_disabled(&item.attrs),
        Item::ExternCrate(item) => is_public(&item.vis) && !is_statically_disabled(&item.attrs),
        Item::Fn(item) => is_public(&item.vis) && !is_statically_disabled(&item.attrs),
        Item::Static(item) => is_public(&item.vis) && !is_statically_disabled(&item.attrs),
        Item::Struct(item) => is_public(&item.vis) && !is_statically_disabled(&item.attrs),
        Item::Trait(item) => is_public(&item.vis) && !is_statically_disabled(&item.attrs),
        Item::TraitAlias(item) => is_public(&item.vis) && !is_statically_disabled(&item.attrs),
        Item::Type(item) => is_public(&item.vis) && !is_statically_disabled(&item.attrs),
        Item::Union(item) => is_public(&item.vis) && !is_statically_disabled(&item.attrs),
        Item::Use(item) => {
            is_public(&item.vis)
                && !is_statically_disabled(&item.attrs)
                && use_tree_exports_name(&item.tree)
        }
        _ => false,
    }
}

fn module_public_contract(
    module: &syn::ItemMod,
    source: &Path,
    base: &Path,
    path_base: &Path,
    visiting: &mut BTreeSet<ModuleContext>,
) -> PublicContract {
    let name = module.ident.unraw().to_string();
    if let Some((_, inline_items)) = &module.content {
        let child_base = base.join(name);
        let key = module_context(source, &child_base);
        if !visiting.insert(key.clone()) {
            return PublicContract::Empty;
        }
        let present =
            items_have_public_contract(inline_items, source, &child_base, &child_base, visiting);
        visiting.remove(&key);
        return contract_status(present);
    }

    let child = external_module_path(module, source, base, path_base)
        .unwrap_or_else(|error| panic!("cannot resolve public module: {error}"));
    let key = module_context(&child.source, &child.base);
    if !visiting.insert(key.clone()) {
        return PublicContract::Empty;
    }
    let text = std::fs::read_to_string(&child.source)
        .unwrap_or_else(|error| panic!("cannot inspect {}: {error}", child.source.display()));
    let syntax = syn::parse_file(&text)
        .unwrap_or_else(|error| panic!("cannot parse {}: {error}", child.source.display()));
    let child_path_base = child
        .source
        .parent()
        .expect("Rust module source must have a parent");
    if is_statically_disabled(&syntax.attrs) {
        visiting.remove(&key);
        return PublicContract::Disabled;
    }
    let present = items_have_public_contract(
        &syntax.items,
        &child.source,
        &child.base,
        child_path_base,
        visiting,
    );
    visiting.remove(&key);
    contract_status(present)
}

fn items_have_public_contract(
    items: &[Item],
    source: &Path,
    base: &Path,
    path_base: &Path,
    visiting: &mut BTreeSet<ModuleContext>,
) -> bool {
    items.iter().any(|item| {
        if is_public_contract_item(item) {
            return true;
        }
        match item {
            Item::Mod(module)
                if is_public(&module.vis) && !is_statically_disabled(&module.attrs) =>
            {
                matches!(
                    module_public_contract(module, source, base, path_base, visiting),
                    PublicContract::Present
                )
            }
            _ => false,
        }
    })
}

fn inspect_public_modules(
    items: &[Item],
    source: &Path,
    base: &Path,
    path_base: &Path,
    empty: &mut BTreeSet<String>,
    inspected: &mut BTreeSet<ModuleContext>,
) {
    if !inspected.insert(module_context(source, base)) {
        return;
    }
    for item in items {
        let Item::Mod(module) = item else {
            continue;
        };
        if is_statically_disabled(&module.attrs) {
            continue;
        }

        let name = module.ident.unraw().to_string();
        let contract = is_public(&module.vis)
            .then(|| module_public_contract(module, source, base, path_base, &mut BTreeSet::new()));
        if contract == Some(PublicContract::Empty) {
            empty.insert(format!("{}: pub mod {name}", source.display()));
        }
        if contract == Some(PublicContract::Disabled) {
            continue;
        }

        if let Some((_, inline_items)) = &module.content {
            let child_base = base.join(name);
            inspect_public_modules(
                inline_items,
                source,
                &child_base,
                &child_base,
                empty,
                inspected,
            );
            continue;
        }

        let child = external_module_path(module, source, base, path_base)
            .unwrap_or_else(|error| panic!("cannot resolve module: {error}"));
        let text = std::fs::read_to_string(&child.source)
            .unwrap_or_else(|error| panic!("cannot inspect {}: {error}", child.source.display()));
        let syntax = syn::parse_file(&text)
            .unwrap_or_else(|error| panic!("cannot parse {}: {error}", child.source.display()));
        if is_statically_disabled(&syntax.attrs) {
            continue;
        }
        let child_path_base = child
            .source
            .parent()
            .expect("Rust module source must have a parent");
        inspect_public_modules(
            &syntax.items,
            &child.source,
            &child.base,
            child_path_base,
            empty,
            inspected,
        );
    }
}

fn inspect_target_root(root: &Path, empty: &mut BTreeSet<String>) {
    let text = std::fs::read_to_string(root)
        .unwrap_or_else(|error| panic!("cannot inspect {}: {error}", root.display()));
    let syntax = syn::parse_file(&text)
        .unwrap_or_else(|error| panic!("cannot parse {}: {error}", root.display()));
    if is_statically_disabled(&syntax.attrs) {
        return;
    }
    let base = root.parent().expect("Cargo target root parent");
    inspect_public_modules(&syntax.items, root, base, base, empty, &mut BTreeSet::new());
}

#[test]
fn public_modules_expose_a_real_contract() {
    let mut empty = BTreeSet::new();
    for package in ifc_packages() {
        for target in package.targets {
            inspect_target_root(target.src_path.as_std_path(), &mut empty);
        }
    }
    assert!(
        empty.is_empty(),
        "public modules without a concrete public item or re-export:\n{}",
        empty.into_iter().collect::<Vec<_>>().join("\n")
    );
}
