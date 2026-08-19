//! Every Rust source file must be reachable from a Cargo target root.
//!
//! Rust quietly ignores an undeclared `.rs` file. This gate asks Cargo for every
//! target root and uses Rust syntax, not text matching, to traverse external
//! modules.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use cargo_metadata::{Metadata, MetadataCommand, Package};
use syn::ext::IdentExt;
use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::{Attribute, Expr, Item, Lit, Meta, Token};

fn metadata() -> Metadata {
    MetadataCommand::new()
        .no_deps()
        .exec()
        .expect("cargo metadata must describe the runtime workspace")
}

fn ifc_packages() -> Vec<Package> {
    let metadata = metadata();
    let root = metadata.workspace_root.as_std_path().join("packages/ifc");
    metadata
        .packages
        .into_iter()
        .filter(|package| {
            package
                .manifest_path
                .as_std_path()
                .parent()
                .is_some_and(|crate_dir| crate_dir.parent() == Some(root.as_path()))
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
            out.insert(path);
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

fn is_statically_disabled(attributes: &[Attribute]) -> bool {
    attributes.iter().any(|attribute| {
        attribute.path().is_ident("cfg")
            && attribute
                .parse_args::<Meta>()
                .ok()
                .and_then(|meta| cfg_value(&meta))
                == Some(false)
    })
}

fn path_override(attributes: &[Attribute]) -> Option<PathBuf> {
    attributes.iter().find_map(|attribute| {
        if !attribute.path().is_ident("path") {
            return None;
        }
        let Meta::NameValue(value) = &attribute.meta else {
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

fn module_base(source: &Path) -> PathBuf {
    let stem = source
        .file_stem()
        .expect("Rust source stem")
        .to_string_lossy();
    if stem == "lib" || stem == "main" || stem == "mod" {
        source.parent().expect("Rust source parent").to_path_buf()
    } else {
        source
            .parent()
            .expect("Rust source parent")
            .join(stem.as_ref())
    }
}

fn visit_items(
    items: &[Item],
    source: &Path,
    base: &Path,
    reached: &mut BTreeSet<PathBuf>,
    missing: &mut Vec<String>,
) {
    for module in items.iter().filter_map(|item| match item {
        Item::Mod(module) if !is_statically_disabled(&module.attrs) => Some(module),
        _ => None,
    }) {
        let name = module.ident.unraw().to_string();
        if let Some((_, inline_items)) = &module.content {
            visit_items(inline_items, source, &base.join(name), reached, missing);
            continue;
        }

        let child = if let Some(path) = path_override(&module.attrs) {
            source.parent().expect("module source parent").join(path)
        } else {
            let flat = base.join(format!("{name}.rs"));
            let nested = base.join(&name).join("mod.rs");
            if flat.is_file() {
                flat
            } else if nested.is_file() {
                nested
            } else {
                missing.push(format!(
                    "{} declares {name}, but neither {} nor {} exists",
                    source.display(),
                    flat.display(),
                    nested.display()
                ));
                continue;
            }
        };
        visit_file(&child, reached, missing);
    }
}

fn visit_file_at_base(
    source: &Path,
    base: PathBuf,
    reached: &mut BTreeSet<PathBuf>,
    missing: &mut Vec<String>,
) {
    let source = source.to_path_buf();
    if !reached.insert(source.clone()) {
        return;
    }
    let text = std::fs::read_to_string(&source)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", source.display()));
    let syntax = syn::parse_file(&text)
        .unwrap_or_else(|error| panic!("cannot parse {}: {error}", source.display()));
    visit_items(&syntax.items, &source, &base, reached, missing);
}

fn visit_file(source: &Path, reached: &mut BTreeSet<PathBuf>, missing: &mut Vec<String>) {
    visit_file_at_base(source, module_base(source), reached, missing);
}

fn visit_target_root(source: &Path, reached: &mut BTreeSet<PathBuf>, missing: &mut Vec<String>) {
    let base = source
        .parent()
        .expect("Cargo target root must have a parent")
        .to_path_buf();
    visit_file_at_base(source, base, reached, missing);
}

#[test]
fn syntax_parser_ignores_comments_and_provably_disabled_modules() {
    let syntax = syn::parse_file(
        r#"
/* mod block_comment; */
// mod line_comment;
mod live;
#[cfg(any())]
mod never;
#[path = "alternate.rs"]
mod redirected;
"#,
    )
    .expect("valid Rust probe");
    let modules: Vec<_> = syntax
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Mod(module) if !is_statically_disabled(&module.attrs) => Some(module),
            _ => None,
        })
        .collect();
    let names: Vec<_> = modules
        .iter()
        .map(|module| module.ident.unraw().to_string())
        .collect();
    assert_eq!(names, ["live", "redirected"]);
    assert_eq!(
        path_override(&modules[1].attrs),
        Some(PathBuf::from("alternate.rs"))
    );
}

#[test]
fn cargo_target_modules_resolve_beside_the_target_root() {
    let dir = std::env::temp_dir().join(format!("nehirde-module-root-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let root = dir.join("custom_target.rs");
    let common = dir.join("common.rs");
    std::fs::write(&root, "mod common;\n").unwrap();
    std::fs::write(&common, "pub fn helper() {}\n").unwrap();

    let mut reached = BTreeSet::new();
    let mut missing = Vec::new();
    visit_target_root(&root, &mut reached, &mut missing);

    assert!(missing.is_empty(), "{missing:#?}");
    assert_eq!(reached, BTreeSet::from([root, common]));
    std::fs::remove_dir_all(dir).unwrap();
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

fn has_public_contract(items: &[Item]) -> bool {
    items.iter().any(|item| match item {
        Item::Const(item) => is_public(&item.vis),
        Item::Enum(item) => is_public(&item.vis),
        Item::ExternCrate(item) => is_public(&item.vis),
        Item::Fn(item) => is_public(&item.vis),
        Item::Mod(item) => is_public(&item.vis),
        Item::Static(item) => is_public(&item.vis),
        Item::Struct(item) => is_public(&item.vis),
        Item::Trait(item) => is_public(&item.vis),
        Item::TraitAlias(item) => is_public(&item.vis),
        Item::Type(item) => is_public(&item.vis),
        Item::Union(item) => is_public(&item.vis),
        Item::Use(item) => is_public(&item.vis),
        _ => false,
    })
}

#[test]
fn public_modules_expose_a_real_contract() {
    let mut empty = Vec::new();
    for package in ifc_packages() {
        let crate_dir = package
            .manifest_path
            .as_std_path()
            .parent()
            .expect("manifest parent");
        let target_roots: BTreeSet<_> = package
            .targets
            .iter()
            .map(|target| target.src_path.as_std_path().to_path_buf())
            .collect();
        let mut sources = BTreeSet::new();
        rust_files(&crate_dir.join("src"), &mut sources);
        for source in sources {
            let text = std::fs::read_to_string(&source).unwrap();
            let syntax = syn::parse_file(&text).unwrap();
            let base = if target_roots.contains(&source) {
                source.parent().expect("Cargo target parent").to_path_buf()
            } else {
                module_base(&source)
            };
            for module in syntax.items.iter().filter_map(|item| match item {
                Item::Mod(module)
                    if is_public(&module.vis) && !is_statically_disabled(&module.attrs) =>
                {
                    Some(module)
                }
                _ => None,
            }) {
                let owned_items;
                let items = if let Some((_, inline)) = &module.content {
                    inline.as_slice()
                } else {
                    let name = module.ident.unraw().to_string();
                    let child = if let Some(path) = path_override(&module.attrs) {
                        source.parent().unwrap().join(path)
                    } else {
                        let flat = base.join(format!("{name}.rs"));
                        let nested = base.join(&name).join("mod.rs");
                        if flat.is_file() {
                            flat
                        } else {
                            nested
                        }
                    };
                    let child_text = std::fs::read_to_string(&child).unwrap_or_else(|error| {
                        panic!("cannot inspect public module {}: {error}", child.display())
                    });
                    owned_items = syn::parse_file(&child_text).unwrap().items;
                    owned_items.as_slice()
                };
                if !has_public_contract(items) {
                    empty.push(format!("{}: pub mod {}", source.display(), module.ident));
                }
            }
        }
    }
    assert!(
        empty.is_empty(),
        "public modules without a public item or re-export:\n{}",
        empty.join("\n")
    );
}
