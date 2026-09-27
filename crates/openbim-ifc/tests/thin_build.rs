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

/// The facade manifest, for reading its dependency and feature tables.
fn manifest() -> String {
    std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"))
        .expect("facade manifest")
}

/// Every optional dependency of the facade: exactly the crates a thin build
/// must not link unless a selected feature asks for them.
///
/// Read from the manifest rather than listed here. The hand-kept list this
/// replaces had fallen nine crates behind the manifest, so a leak of any of
/// those into the thin build would have passed.
fn optional_dependencies() -> Vec<String> {
    let manifest = manifest();
    let mut in_dependencies = false;
    let mut out = Vec::new();
    for line in manifest.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_dependencies = line == "[dependencies]";
            continue;
        }
        if in_dependencies && line.contains("optional = true") {
            if let Some((name, _)) = line.split_once('=') {
                out.push(name.trim().to_owned());
            }
        }
    }
    assert!(
        out.len() >= 20,
        "expected every facade domain as an optional dependency, found {out:?}"
    );
    out
}

/// The crates a feature enables directly (`dep:name` values).
fn enabled_by(feature: &str) -> Vec<String> {
    let manifest = manifest();
    let prefix = format!("{feature} = [");
    let line = manifest
        .lines()
        .find(|line| line.starts_with(&prefix))
        .unwrap_or_else(|| panic!("feature `{feature}` is not a one-line table entry"));
    line.split('"')
        .filter_map(|value| value.strip_prefix("dep:"))
        .map(str::to_owned)
        .collect()
}

/// Optional dependencies the thin build must not link.
fn domain_crates() -> Vec<String> {
    let allowed = enabled_by("step");
    assert_eq!(
        allowed,
        ["ifc-step"],
        "the thin build is defined as the STEP codec"
    );
    optional_dependencies()
        .into_iter()
        .filter(|dep| !allowed.contains(dep))
        .collect()
}

/// Does the resolved tree contain this crate?
fn links(tree: &str, crate_name: &str) -> bool {
    tree.lines()
        .filter_map(|l| l.split_whitespace().next())
        .any(|name| name == crate_name)
}

fn linked_with_prefix<'a>(tree: &'a str, prefix: &str) -> Vec<&'a str> {
    tree.lines()
        .filter_map(|line| line.split_whitespace().next())
        .filter(|name| name.starts_with(prefix))
        .collect()
}

/// The thin build must not drag in a single domain crate.
#[test]
fn thin_build_excludes_every_domain_crate() {
    let tree = dependency_tree("step");

    for forbidden in &domain_crates() {
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

    for forbidden in &domain_crates() {
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
    let axiolid = linked_with_prefix(&tree, "axiolid-");
    assert!(
        axiolid.is_empty() && !links(&tree, "glam"),
        "thin build links geometry packages {axiolid:?}; a file-mover should compile no geometry\n{tree}"
    );
}

/// Selecting a 2D representation must not link the geometry kernel.
///
/// Regression for openbimrs/ifc#2. A drawing consumer needs contexts, plan
/// selection, placements and units -- all slot reads over `ifc-model`. Before
/// this split, asking for any of it linked all eight `axiolid-*` crates plus
/// `glam`, because `ifc-geometry` was one undivided crate.
#[test]
fn selecting_a_plan_representation_links_no_geometry_kernel() {
    let tree = dependency_tree("step,geometry-select");

    assert!(
        links(&tree, "ifc-geometry"),
        "geometry-select must still link the selectors"
    );
    let axiolid = linked_with_prefix(&tree, "axiolid-");
    assert!(
        axiolid.is_empty() && !links(&tree, "glam"),
        "geometry-select links geometry packages {axiolid:?}; a 2D consumer should compile no \
         kernel. Check that `ifc-geometry` is taken with \
         `default-features = false`.\n{tree}"
    );
}

/// The full `geometry` feature must still deliver lowering.
///
/// The inverse of the test above: a feature table that turned the kernel off
/// everywhere would pass that one while breaking every 3D consumer.
#[test]
fn the_geometry_feature_still_links_the_kernel() {
    let tree = dependency_tree("step,geometry");

    for required in ["axiolid-core", "axiolid-model"] {
        assert!(
            links(&tree, required),
            "the `geometry` feature lost {required}; lowering cannot work.\n{tree}"
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

/// With every feature on, `compiled_features()` reports every feature that
/// enables a crate: pure bundles (`codecs`, `domains`, `full`), whose values
/// are only other features, may be absent. The list is hand-kept `#[cfg]`
/// pushes, and it had silently lost six features before this test existed.
#[cfg(feature = "full")]
#[test]
fn compiled_features_names_every_feature_that_enables_a_crate() {
    let manifest = manifest();
    let features: Vec<(String, String)> = manifest
        .split("[features]")
        .nth(1)
        .and_then(|rest| rest.split("\n[").next())
        .expect("a [features] table")
        .split('\n')
        .fold(Vec::new(), |mut entries: Vec<(String, String)>, line| {
            match line.split_once(" = [") {
                Some((name, values)) if !line.starts_with([' ', '#']) => {
                    entries.push((name.trim().to_owned(), values.to_owned()))
                }
                _ => {
                    if let Some(last) = entries.last_mut() {
                        last.1.push_str(line);
                    }
                }
            }
            entries
        });
    let compiled = ifc::compiled_features();
    let mut checked = 0;
    for (name, values) in &features {
        if name == "default" {
            continue;
        }
        let enables_crate = values.contains("dep:") || values.contains('/');
        if enables_crate {
            checked += 1;
            assert!(
                compiled.contains(&name.as_str()),
                "compiled_features() does not report `{name}`; add it to src/feature_report.rs"
            );
        }
        assert!(
            compiled
                .iter()
                .all(|c| features.iter().any(|(n, _)| n == c)),
            "compiled_features() reports a feature the manifest does not declare"
        );
    }
    assert!(
        checked >= 25,
        "expected every facade feature, checked {checked}"
    );
}
