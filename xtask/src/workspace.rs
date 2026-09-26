//! The workspace as cargo describes it at run time.
//!
//! The root is resolved from `cargo metadata`, never from a compile-time
//! `CARGO_MANIFEST_DIR`: this repository is checked out in several worktrees
//! sharing one build cache, and a baked-in path would make one checkout
//! generate another checkout's docs.

use std::collections::BTreeMap;
use std::path::PathBuf;

use cargo_metadata::{DependencyKind, Metadata, MetadataCommand};

/// Workspace members that are tooling rather than published crates.
pub(crate) const TOOLING: &[&str] = &["xtask"];

/// Declared maturity of a crate, from `[package.metadata.openbim] status`.
pub(crate) const STATUSES: &[&str] = &["implemented", "partial", "scaffold"];

/// Crate groups, in the order the docs present them, with their titles.
pub(crate) const GROUPS: &[(&str, &str)] = &[
    ("facade", "Facade"),
    ("core", "Model, codecs, schema, authoring and validation"),
    ("domain", "Domain views"),
    ("geometry", "Geometry, georeferencing and alignment"),
    ("binding", "Language bindings"),
];

/// The workspace: its root and its packages.
pub(crate) struct Workspace {
    pub(crate) root: PathBuf,
    pub(crate) metadata: Metadata,
}

/// One library crate of the family, with its declared documentation metadata.
pub(crate) struct Crate {
    pub(crate) name: String,
    pub(crate) version: String,
    pub(crate) description: String,
    /// `false` for `publish = false`.
    pub(crate) publish: bool,
    /// The crate directory, relative to the workspace root.
    pub(crate) dir: String,
    pub(crate) status: String,
    pub(crate) group: String,
    /// `[features]`, without `default`.
    pub(crate) features: BTreeMap<String, Vec<String>>,
    /// `default` feature values.
    pub(crate) default_features: Vec<String>,
    /// `[package.metadata.openbim.features]` descriptions.
    pub(crate) feature_notes: BTreeMap<String, String>,
    /// Workspace members this crate depends on at run time, sorted.
    pub(crate) internal_deps: Vec<String>,
    /// The library target: its name (rustdoc directory) and root source file.
    pub(crate) lib: Option<(String, PathBuf)>,
    /// `rust-version`, the minimum supported Rust.
    pub(crate) rust_version: Option<String>,
}

impl Workspace {
    pub(crate) fn load() -> Result<Self, String> {
        let metadata = MetadataCommand::new()
            .no_deps()
            .exec()
            .map_err(|error| format!("cargo metadata failed: {error}"))?;
        Ok(Self {
            root: metadata.workspace_root.clone().into_std_path_buf(),
            metadata,
        })
    }

    /// Names of every publishable member, sorted.
    ///
    /// `publish = false` is `Some([])` in cargo metadata; a new crate is
    /// picked up the moment it joins the workspace.
    pub(crate) fn publishable(&self) -> Vec<String> {
        let mut names: Vec<String> = self
            .metadata
            .packages
            .iter()
            .filter(|package| package.publish.as_ref().is_none_or(|to| !to.is_empty()))
            .map(|package| package.name.to_string())
            .collect();
        names.sort();
        names
    }

    /// Every non-tooling member, sorted by name, with validated metadata.
    ///
    /// A crate without `[package.metadata.openbim]`, or with a status or group
    /// the docs do not know, is an error: the docs would otherwise silently
    /// omit it or invent its standing.
    pub(crate) fn crates(&self) -> Result<Vec<Crate>, String> {
        let members: Vec<String> = self
            .metadata
            .packages
            .iter()
            .map(|package| package.name.to_string())
            .collect();
        let mut out = Vec::new();
        for package in &self.metadata.packages {
            let name = package.name.to_string();
            if TOOLING.contains(&name.as_str()) {
                continue;
            }
            let declared = package.metadata.get("openbim").ok_or_else(|| {
                format!("{name}: add [package.metadata.openbim] with `status` and `group`")
            })?;
            let text = |key: &str| {
                declared
                    .get(key)
                    .and_then(|value| value.as_str())
                    .map(str::to_owned)
                    .ok_or_else(|| format!("{name}: [package.metadata.openbim] lacks `{key}`"))
            };
            let status = text("status")?;
            if !STATUSES.contains(&status.as_str()) {
                return Err(format!(
                    "{name}: status `{status}` is not one of {STATUSES:?}"
                ));
            }
            let group = text("group")?;
            if !GROUPS.iter().any(|(key, _)| *key == group) {
                return Err(format!("{name}: group `{group}` is not a known group"));
            }
            let feature_notes = declared
                .get("features")
                .and_then(|value| value.as_object())
                .map(|table| {
                    table
                        .iter()
                        .filter_map(|(k, v)| Some((k.clone(), v.as_str()?.to_owned())))
                        .collect()
                })
                .unwrap_or_default();
            let mut features: BTreeMap<String, Vec<String>> = package
                .features
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            let default_features = features.remove("default").unwrap_or_default();
            let mut internal_deps: Vec<String> = package
                .dependencies
                .iter()
                .filter(|dep| dep.kind == DependencyKind::Normal)
                .map(|dep| dep.name.clone())
                .filter(|dep| members.contains(dep))
                .collect();
            internal_deps.sort();
            internal_deps.dedup();
            let dir = package
                .manifest_path
                .parent()
                .and_then(|dir| dir.strip_prefix(&self.metadata.workspace_root).ok())
                .map(|dir| dir.to_string())
                .unwrap_or_else(|| name.clone());
            let lib = package
                .targets
                .iter()
                .find(|target| {
                    target
                        .kind
                        .iter()
                        .any(|kind| matches!(kind.to_string().as_str(), "lib" | "rlib" | "cdylib"))
                })
                .map(|target| {
                    (
                        target.name.replace('-', "_"),
                        target.src_path.clone().into_std_path_buf(),
                    )
                });
            out.push(Crate {
                rust_version: package.rust_version.as_ref().map(|v| v.to_string()),
                lib,
                version: package.version.to_string(),
                description: package.description.clone().unwrap_or_default(),
                publish: package.publish.as_ref().is_none_or(|to| !to.is_empty()),
                dir,
                status,
                group,
                features,
                default_features,
                feature_notes,
                internal_deps,
                name,
            });
        }
        out.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(out)
    }

    /// `references/ifc-spec/<rel>`, in either checkout layout.
    ///
    /// Standalone the root holds `references/`; as a submodule of the
    /// openbim superproject it sits three levels up.
    pub(crate) fn spec(&self, rel: &str) -> Option<PathBuf> {
        ["references/ifc-spec", "../../../references/ifc-spec"]
            .into_iter()
            .map(|base| self.root.join(base).join(rel))
            .find(|path| path.exists())
    }
}
