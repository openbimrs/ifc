//! A bring-your-own-kernel build links no reference backend.
//!
//! The `compile` feature is the seam; `compile-reference-backend` is the
//! engine. Keeping them separate is what lets a CGAL or OCCT consumer pay
//! for contracts only -- but a stray non-optional dependency, or a feature
//! edge added later, would silently relink the reference implementation and
//! no ordinary test would notice, because the reference build links it
//! anyway.
//!
//! This inspects the **resolved dependency graph**, the same technique as
//! `kernel_free_build.rs`.

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
        .any(|line| line.split_whitespace().next() == Some(crate_name))
}

/// The engine crates, as opposed to the contracts that describe them.
const REFERENCE_ENGINE: &[&str] = &["axiolid-mesh-compile", "axiolid-mesh-boolean-boolmesh"];

/// The contracts a caller implements to bring their own kernel.
const CONTRACTS: &[&str] = &["axiolid-mesh-compile-contract", "axiolid-contracts"];

#[test]
fn the_seam_alone_links_contracts_but_no_engine() {
    let tree = dependency_tree(&["--no-default-features", "--features", "compile"]);

    for engine in REFERENCE_ENGINE {
        assert!(
            !links(&tree, engine),
            "`compile` must not link {engine}: a bring-your-own-kernel build \
             pays for contracts only. Move the dependency under \
             `compile-reference-backend`."
        );
    }
    for contract in CONTRACTS {
        assert!(
            links(&tree, contract),
            "`compile` must link {contract}, or a caller cannot implement a backend"
        );
    }
}

#[test]
fn the_reference_feature_adds_the_engine() {
    let tree = dependency_tree(&[
        "--no-default-features",
        "--features",
        "compile-reference-backend",
    ]);

    for engine in REFERENCE_ENGINE {
        assert!(
            links(&tree, engine),
            "`compile-reference-backend` must link {engine}"
        );
    }
}

/// The default build compiles nothing: no contracts, no engine.
#[test]
fn a_default_build_links_neither() {
    let tree = dependency_tree(&[]);

    for krate in REFERENCE_ENGINE.iter().chain(CONTRACTS.iter()) {
        assert!(
            !links(&tree, krate),
            "the default build must not link {krate}; compilation is opt-in per ADR 0004"
        );
    }
}
