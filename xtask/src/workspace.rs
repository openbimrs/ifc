//! The workspace as cargo describes it at run time.
//!
//! The root is resolved from `cargo metadata`, never from a compile-time
//! `CARGO_MANIFEST_DIR`: this repository is checked out in several worktrees
//! sharing one build cache, and a baked-in path would make one checkout
//! generate another checkout's docs.

use std::path::PathBuf;

use cargo_metadata::{Metadata, MetadataCommand};

/// The workspace: its root and its packages.
pub(crate) struct Workspace {
    pub(crate) root: PathBuf,
    pub(crate) metadata: Metadata,
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
