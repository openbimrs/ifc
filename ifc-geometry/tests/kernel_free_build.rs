//! Proof that the kernel-free build is real, not decorative.
//!
//! A 2D consumer selects a representation, applies a transform and reads
//! units. None of that needs a geometry kernel, but a single unconditional
//! `use axiolid_core` anywhere in the crate silently relinks all of it --
//! and nothing in a normal test run would notice, because the default build
//! links the kernel anyway.
//!
//! This checks the **resolved dependency graph** under
//! `--no-default-features`, not the manifest: an optional dependency can be
//! re-enabled by accident through a feature edge, and only the resolver
//! knows. Same technique as `openbim-ifc`'s `thin_build.rs`.

use std::process::Command;

/// Every crate linked into `ifc-geometry` for a given feature selection.
fn dependency_tree(extra: &[&str]) -> String {
    let manifest = concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml");
    let mut cmd = Command::new(env!("CARGO"));
    cmd.args([
        "tree",
        "--manifest-path",
        manifest,
        "--edges",
        "normal",
        "--prefix",
        "none",
    ]);
    cmd.args(extra);
    let out = cmd.output().expect("cargo tree should run");
    assert!(
        out.status.success(),
        "cargo tree failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).expect("tree output is utf-8")
}

fn links(tree: &str, crate_name: &str) -> bool {
    tree.lines()
        .filter_map(|line| line.split_whitespace().next())
        .any(|name| name == crate_name)
}

/// The neutral geometry crates, plus the math library they all pull.
///
/// `axiolid-mesh` and `axiolid-surface` are not direct dependencies but
/// arrive transitively through `axiolid-model`, so a check that only listed
/// the direct ones would miss most of the weight.
const KERNEL_CRATES: &[&str] = &[
    "axiolid-core",
    "axiolid-curve",
    "axiolid-mesh",
    "axiolid-model",
    "axiolid-primitive",
    "axiolid-profile",
    "axiolid-surface",
    "axiolid-topology",
    "glam",
];

/// Without `lowering`, not one geometry crate may be linked.
#[test]
fn the_kernel_free_build_links_no_geometry_crate() {
    let tree = dependency_tree(&["--no-default-features"]);

    for forbidden in KERNEL_CRATES {
        assert!(
            !links(&tree, forbidden),
            "a kernel-free build links {forbidden}. Something outside `lower` \
             uses the neutral vocabulary unconditionally; gate it behind \
             `#[cfg(feature = \"lowering\")]`.\n{tree}"
        );
    }

    // ...while keeping what representation selection actually needs.
    assert!(links(&tree, "ifc-model"), "the model is not optional");
    assert!(
        links(&tree, "ifc-schema"),
        "schema queries are not optional"
    );
}

/// With `lowering`, the neutral vocabulary must be present.
///
/// The inverse assertion: a feature table that gated *everything* off would
/// pass the test above while making the crate useless.
#[test]
fn the_default_build_still_links_the_neutral_vocabulary() {
    let tree = dependency_tree(&[]);

    for required in ["axiolid-core", "axiolid-model"] {
        assert!(
            links(&tree, required),
            "the default build lost {required}; lowering cannot work.\n{tree}"
        );
    }
}
