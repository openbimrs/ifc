use super::*;

struct TestTree {
    root: PathBuf,
}

impl TestTree {
    fn new(name: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "nehirde-module-reachability-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("create test tree");
        Self { root }
    }

    fn path(&self, relative: &str) -> PathBuf {
        self.root.join(relative)
    }

    fn write(&self, relative: &str, contents: &str) {
        let path = self.path(relative);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("create test parent");
        }
        std::fs::write(path, contents).expect("write test source");
    }
}

impl Drop for TestTree {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
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
fn nested_path_overrides_resolve_from_the_inline_module_base() {
    let dir = std::env::temp_dir().join(format!("nehirde-nested-path-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("outer")).unwrap();
    let root = dir.join("custom_target.rs");
    let redirected = dir.join("outer/redirected.rs");
    std::fs::write(
        &root,
        "mod outer { #[path = \"redirected.rs\"] mod child; }\n",
    )
    .unwrap();
    std::fs::write(&redirected, "pub fn helper() {}\n").unwrap();

    let mut reached = BTreeSet::new();
    let mut missing = Vec::new();
    visit_target_root(&root, &mut reached, &mut missing);

    assert!(missing.is_empty(), "{missing:#?}");
    assert_eq!(reached, BTreeSet::from([root, redirected]));
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn shared_source_is_traversed_in_each_module_context() {
    let tree = TestTree::new("shared-context");
    tree.write("root.rs", "mod nested;\n");
    tree.write("nested.rs", "mod child;\n");
    tree.write("child.rs", "fn target_child() {}\n");
    tree.write("nested/child.rs", "fn module_child() {}\n");

    let mut reached = BTreeSet::new();
    let mut missing = Vec::new();
    visit_target_root(&tree.path("nested.rs"), &mut reached, &mut missing);
    visit_target_root(&tree.path("root.rs"), &mut reached, &mut missing);

    assert!(
        missing.is_empty(),
        "unexpected missing modules: {missing:#?}"
    );
    assert!(reached.contains(&tree.path("child.rs").canonicalize().unwrap()));
    assert!(reached.contains(&tree.path("nested/child.rs").canonicalize().unwrap()));
}

#[test]
fn path_aliases_share_one_canonical_source_identity() {
    let tree = TestTree::new("path-alias");
    tree.write("root.rs", "#[path = \"alias/../real.rs\"] mod real;\n");
    std::fs::create_dir_all(tree.path("alias")).unwrap();
    tree.write("real.rs", "fn marker() {}\n");

    let mut reached = BTreeSet::new();
    let mut missing = Vec::new();
    visit_target_root(&tree.path("root.rs"), &mut reached, &mut missing);

    assert!(
        missing.is_empty(),
        "unexpected missing modules: {missing:#?}"
    );
    assert!(reached.contains(&tree.path("real.rs").canonicalize().unwrap()));
}

#[cfg(unix)]
#[test]
fn symlinked_module_descendants_resolve_beside_the_declared_path() {
    use std::os::unix::fs::symlink;

    let tree = TestTree::new("path-symlink");
    tree.write("root.rs", "#[path = \"alias.rs\"] mod linked;\n");
    tree.write("target/real.rs", "mod child;\n");
    tree.write("child.rs", "fn marker() {}\n");
    symlink("target/real.rs", tree.path("alias.rs")).unwrap();

    let mut reached = BTreeSet::new();
    let mut missing = Vec::new();
    visit_target_root(&tree.path("root.rs"), &mut reached, &mut missing);

    assert!(
        missing.is_empty(),
        "unexpected missing modules: {missing:#?}"
    );
    assert!(reached.contains(&tree.path("child.rs").canonicalize().unwrap()));
}

#[test]
fn cyclic_path_graph_produces_bounded_public_diagnostics() {
    let tree = TestTree::new("path-cycle");
    tree.write("root.rs", "#[path = \"cycle.rs\"] pub mod cycle;\n");
    tree.write("cycle.rs", "#[path = \"cycle.rs\"] pub mod again;\n");

    let mut empty = BTreeSet::new();
    inspect_target_root(&tree.path("root.rs"), &mut empty);

    assert_eq!(empty.len(), 2, "unexpected cycle diagnostics: {empty:#?}");
}

#[test]
fn dot_segment_path_cycle_has_one_effective_context() {
    let tree = TestTree::new("dot-segment-path-cycle");
    tree.write("root.rs", "#[path = \"cycle.rs\"] pub mod cycle;\n");
    tree.write("cycle.rs", "#[path = \"sub/../cycle.rs\"] pub mod again;\n");
    std::fs::create_dir_all(tree.path("sub")).unwrap();

    let mut empty = BTreeSet::new();
    inspect_target_root(&tree.path("root.rs"), &mut empty);

    assert_eq!(empty.len(), 2, "unexpected cycle diagnostics: {empty:#?}");
}

#[test]
fn zero_export_use_does_not_count_as_a_public_contract() {
    let tree = TestTree::new("empty-use");
    tree.write(
        "root.rs",
        r#"
pub mod outer {
    mod empty {}
    #[allow(unused_imports)]
    pub use self::empty::{};
}
"#,
    );
    let mut empty = BTreeSet::new();
    inspect_target_root(&tree.path("root.rs"), &mut empty);
    assert!(empty.iter().any(|item| item.ends_with("pub mod outer")));
}

#[test]
fn out_of_line_path_overrides_resolve_beside_the_source_file() {
    let tree = TestTree::new("out-of-line-path");
    tree.write("root.rs", "mod outer;\n");
    tree.write("outer.rs", "#[path = \"redirected.rs\"]\nmod child;\n");
    tree.write("redirected.rs", "fn marker() {}\n");

    let mut reached = BTreeSet::new();
    let mut missing = Vec::new();
    visit_target_root(&tree.path("root.rs"), &mut reached, &mut missing);

    assert!(
        missing.is_empty(),
        "unexpected missing modules: {missing:#?}"
    );
    assert!(reached.contains(&tree.path("redirected.rs")));
}

#[test]
fn path_overridden_module_descendants_resolve_beside_the_overridden_source() {
    let tree = TestTree::new("path-descendants");
    tree.write("root.rs", "#[path = \"weird/renamed.rs\"]\nmod outer;\n");
    tree.write("weird/renamed.rs", "mod child;\n");
    tree.write("weird/child.rs", "fn marker() {}\n");

    let mut reached = BTreeSet::new();
    let mut missing = Vec::new();
    visit_target_root(&tree.path("root.rs"), &mut reached, &mut missing);

    assert!(
        missing.is_empty(),
        "unexpected missing modules: {missing:#?}"
    );
    assert!(reached.contains(&tree.path("weird/child.rs")));
}

#[test]
fn statically_active_cfg_attr_controls_cfg_and_path() {
    let tree = TestTree::new("cfg-attr");
    tree.write(
        "root.rs",
        r#"
#[cfg_attr(all(), path = "alternate.rs")]
mod redirected;
#[cfg_attr(any(), path = "missing.rs")]
mod defaulted;
pub mod empty {
    #[cfg_attr(all(), cfg(any()))]
    pub struct Ghost;
}
"#,
    );
    tree.write("alternate.rs", "fn marker() {}\n");
    tree.write("defaulted.rs", "fn marker() {}\n");

    let mut reached = BTreeSet::new();
    let mut missing = Vec::new();
    visit_target_root(&tree.path("root.rs"), &mut reached, &mut missing);
    assert!(
        missing.is_empty(),
        "unexpected missing modules: {missing:#?}"
    );
    assert!(reached.contains(&tree.path("alternate.rs")));
    assert!(reached.contains(&tree.path("defaulted.rs")));

    let mut empty = BTreeSet::new();
    inspect_target_root(&tree.path("root.rs"), &mut empty);
    assert!(empty.iter().any(|name| name.ends_with("pub mod empty")));
}

#[test]
fn file_level_cfg_disables_external_module_contents() {
    let tree = TestTree::new("file-cfg");
    tree.write("root.rs", "pub mod direct;\npub mod attributed;\n");
    tree.write(
        "direct.rs",
        "#![cfg(any())]\nmod missing;\npub struct Ghost;\n",
    );
    tree.write(
        "attributed.rs",
        "#![cfg_attr(all(), cfg(any()))]\nmod missing;\npub struct Ghost;\n",
    );

    let mut reached = BTreeSet::new();
    let mut missing = Vec::new();
    visit_target_root(&tree.path("root.rs"), &mut reached, &mut missing);
    assert!(
        missing.is_empty(),
        "unexpected missing modules: {missing:#?}"
    );

    let mut empty = BTreeSet::new();
    inspect_target_root(&tree.path("root.rs"), &mut empty);
    assert!(
        empty.is_empty(),
        "file-level cfg removes the module item: {empty:#?}"
    );
}

#[test]
fn recursive_public_contract_check_rejects_empty_namespaces() {
    let dir = std::env::temp_dir().join(format!("nehirde-public-api-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("api")).unwrap();
    let root = dir.join("api/custom_root.rs");
    std::fs::write(
        &root,
        "pub mod outer { pub mod empty {} }\n\
         pub mod disabled { #[cfg(any())] pub struct Ghost; }\n",
    )
    .unwrap();

    let mut empty = BTreeSet::new();
    inspect_target_root(&root, &mut empty);

    assert!(empty.iter().any(|item| item.ends_with("pub mod outer")));
    assert!(empty.iter().any(|item| item.ends_with("pub mod empty")));
    assert!(empty.iter().any(|item| item.ends_with("pub mod disabled")));
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn recursive_public_contract_check_accepts_a_nested_contract() {
    let syntax = syn::parse_file("pub mod outer { pub mod inner { pub struct Contract; } }")
        .expect("valid nested public contract");
    let mut empty = BTreeSet::new();
    inspect_public_modules(
        &syntax.items,
        Path::new("virtual.rs"),
        Path::new("."),
        Path::new("."),
        &mut empty,
        &mut BTreeSet::new(),
    );
    assert!(empty.is_empty(), "{empty:#?}");
}
