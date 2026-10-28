//! Every crate published to crates.io ships the dual-licence texts.
//!
//! The repository is `AGPL-3.0-or-later OR LicenseRef-Commercial` (LICENSING.md).
//! A crates.io archive contains only the crate directory, so each published
//! crate carries byte-identical copies of the root `LICENSE`, `NOTICE` and
//! `LICENSES/LicenseRef-Commercial.txt`, and its manifest names one of the
//! two allowed expressions (the second where the artifact embeds the
//! CC-BY-ND-4.0 template catalog).

use std::path::{Path, PathBuf};

use cargo_metadata::{MetadataCommand, Package};

const DUAL: &str = "AGPL-3.0-or-later OR LicenseRef-Commercial";
const DUAL_WITH_CATALOG: &str = "(AGPL-3.0-or-later OR LicenseRef-Commercial) AND CC-BY-ND-4.0";
/// `ifc-template-catalog` and the catalog-embedding bindings ship the CC-BY-ND
/// texts as well.
const CATALOG_TEXT: &str = "LICENSES/CC-BY-ND-4.0.txt";
const COPIES: &[&str] = &["LICENSE", "NOTICE", "LICENSES/LicenseRef-Commercial.txt"];
/// Fewer than this many published crates means the selection matched nothing
/// real (the workspace has 28 today).
const MIN_PUBLISHED: usize = 25;

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

/// Published crates: located under `crates/`, selected by name, and not
/// `publish = false`.
fn published(root: &Path) -> Vec<Package> {
    let metadata = MetadataCommand::new()
        .no_deps()
        .exec()
        .expect("cargo metadata");
    let crates = root.join("crates");
    metadata
        .packages
        .into_iter()
        .filter(|p| {
            let dir = p
                .manifest_path
                .as_std_path()
                .parent()
                .unwrap()
                .to_path_buf();
            dir.parent() == Some(crates.as_path())
                && (p.name.starts_with("ifc-") || p.name.starts_with("openbim-ifc"))
                && p.publish
                    .as_ref()
                    .is_none_or(|registries| !registries.is_empty())
        })
        .collect()
}

#[test]
fn published_crates_ship_the_licence_texts() {
    let root = repository_root();
    let crates = published(&root);
    assert!(
        crates.len() >= MIN_PUBLISHED,
        "found only {} published crates",
        crates.len()
    );
    let mut failures = Vec::new();
    for package in &crates {
        let dir = package.manifest_path.as_std_path().parent().unwrap();
        for copy in COPIES {
            match (
                std::fs::read(root.join(copy)),
                std::fs::read(dir.join(copy)),
            ) {
                (Ok(want), Ok(have)) if want == have => {}
                (Ok(_), Ok(_)) => {
                    failures.push(format!("{}: {copy} differs from the root", package.name))
                }
                (_, Err(_)) => failures.push(format!("{}: {copy} is missing", package.name)),
                (Err(e), _) => panic!("root {copy}: {e}"),
            }
        }
        let license = package.license.as_deref().unwrap_or("");
        if license != DUAL && license != DUAL_WITH_CATALOG {
            failures.push(format!(
                "{}: license `{license}` is not an allowed expression",
                package.name
            ));
        }
        if license.contains("CC-BY-ND-4.0") && !dir.join(CATALOG_TEXT).is_file() {
            failures.push(format!("{}: {CATALOG_TEXT} is missing", package.name));
        }
    }
    assert!(
        failures.is_empty(),
        "licence files:\n{}",
        failures.join("\n")
    );
}
