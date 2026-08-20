//! Proof that feature gating is real, not decorative.
//!
//! The design claim is that a thin application compiles no domain code while
//! still round-tripping domain data losslessly. This checks the **resolved
//! dependency graph** rather than trusting the manifest, because a manifest
//! can declare an optional dependency that a feature accidentally enables.
//!
//! Implementation note: `cargo metadata` lists every workspace package
//! regardless of features, so reading the package list proves nothing. The
//! real answer is in `resolve.nodes` — the actual edges for the selected
//! feature set — which is what `cargo tree` prints.

use std::process::Command;

/// Crate names actually linked into `-p ifc` under an explicit feature set.
fn dependency_tree(features: &str) -> String {
    tree_with(&["--no-default-features", "--features", features])
}

/// Crate names linked into `-p ifc` with its **default** features.
///
/// Checked separately from [`dependency_tree`]: a domain accidentally added to
/// `default` would be invisible to a test that always passes
/// `--no-default-features`, which is exactly the mistake most likely to happen
/// while editing the feature table.
fn default_tree() -> String {
    tree_with(&[])
}

fn tree_with(extra: &[&str]) -> String {
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

/// Every domain crate, for exclusion checks.
const DOMAIN_CRATES: &[&str] = &[
    "ifc-cost",
    "ifc-schedule",
    "ifc-properties",
    "ifc-material",
    "ifc-classification",
    "ifc-structural",
    "ifc-resource",
    "ifc-systems",
    "ifc-style",
    "ifc-validate",
    "ifc-geometry",
    "ifc-georef",
    "ifc-alignment",
];

/// Does the resolved tree contain this crate?
fn links(tree: &str, crate_name: &str) -> bool {
    tree.lines()
        .filter_map(|l| l.split_whitespace().next())
        .any(|name| name == crate_name)
}

/// The thin build must not drag in a single domain crate.
#[test]
fn thin_build_excludes_every_domain_crate() {
    let tree = dependency_tree("step");

    for forbidden in DOMAIN_CRATES {
        assert!(
            !links(&tree, forbidden),
            "thin build links {forbidden}; feature gating is broken.\n{tree}"
        );
    }

    // ...while still having what it needs to read and write files.
    assert!(links(&tree, "ifc-model"), "thin build lost the model");
    assert!(links(&tree, "ifc-step"), "thin build lost the codec");
}

/// The **default** feature set must stay thin.
///
/// Separate from the explicit-feature test above: `default` is what a consumer
/// gets by typing `ifc = "0.1"`, so a domain leaking into it silently makes
/// every downstream build fat. This is the mutation that a
/// `--no-default-features` test cannot see.
#[test]
fn default_features_pull_in_no_domain_crate() {
    let tree = default_tree();

    for forbidden in DOMAIN_CRATES {
        assert!(
            !links(&tree, forbidden),
            "the DEFAULT feature set links {forbidden}. `default` must stay thin \
             -- move it to an opt-in feature.\n{tree}"
        );
    }
    assert!(
        links(&tree, "ifc-step"),
        "default should still be able to read files"
    );
}

/// Selecting one domain must not drag in its siblings.
#[test]
fn selecting_cost_does_not_pull_in_unrelated_domains() {
    let tree = dependency_tree("step,cost");

    assert!(
        links(&tree, "ifc-cost"),
        "cost feature did not link ifc-cost"
    );
    for unrelated in [
        "ifc-structural",
        "ifc-style",
        "ifc-alignment",
        "ifc-geometry",
    ] {
        assert!(
            !links(&tree, unrelated),
            "enabling `cost` pulled in {unrelated}\n{tree}"
        );
    }
}

/// A geometry-free build must not compile the geometry kernel at all — that is
/// the payoff of keeping geometry out of the model.
#[test]
fn thin_build_compiles_no_geometry_kernel() {
    let tree = dependency_tree("step");
    for axiolid in ["axiolid-kernel", "axiolid-core", "axiolid-mesh", "glam"] {
        assert!(
            !links(&tree, axiolid),
            "thin build links {axiolid}; a file-mover should compile no geometry"
        );
    }
}

/// The model must never depend on a codec: that inversion would make ifcXML a
/// second parallel stack and break cross-format conversion.
#[test]
fn the_model_does_not_depend_on_any_codec() {
    let manifest = concat!(env!("CARGO_MANIFEST_DIR"), "/../ifc-model/Cargo.toml");
    let text = std::fs::read_to_string(manifest).expect("ifc-model manifest");
    let body: String = text
        .lines()
        .map(|l| l.split('#').next().unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n");

    for codec in ["ifc-step", "ifc-xml", "ifc-json"] {
        assert!(
            !body.contains(codec),
            "ifc-model depends on the {codec} codec. Codecs depend on the model, \
             not the reverse -- otherwise every new serialization needs its own \
             parallel data model. See docs/adr/0006."
        );
    }
}

/// The facade reports what it was built with.
#[test]
fn compiled_features_reflects_the_build() {
    let features = ifc::compiled_features();
    assert!(features.contains(&"step"), "default build should have step");
    #[cfg(not(feature = "cost"))]
    assert!(!features.contains(&"cost"));
}
