//! What each crate has released, and where it can be installed from.
//!
//! The released version is the newest dated section of the crate's own
//! `CHANGELOG.md`. The release commit is what dates that section, so this is a
//! property of the commit, not of the moment the docs are built: deriving it
//! from git tags instead would make every release (the tag is pushed after the
//! release commit merges) turn `main`'s docs stale and fail every open PR.
//! Whether each release is actually tagged is a separate check.

use super::changelog::{compare_versions, heading};
use crate::text::splitlines;
use crate::workspace::{Crate, Workspace};

/// The newest release a crate's changelog records.
pub(crate) struct Release {
    pub(crate) version: String,
    pub(crate) date: String,
}

/// A registry a crate is published to.
pub(crate) struct Registry {
    /// `crates.io`, `npm`, `PyPI`, `NuGet` or `GitHub release`.
    pub(crate) kind: &'static str,
    /// The package name there, which need not equal the crate name.
    pub(crate) package: String,
    pub(crate) url: String,
}

/// The newest dated `## [x.y.z] - date` section, if any.
pub(crate) fn latest(workspace: &Workspace, krate: &Crate) -> Result<Option<Release>, String> {
    let path = workspace.root.join(&krate.dir).join("CHANGELOG.md");
    let text = std::fs::read_to_string(&path)
        .map_err(|error| format!("{}/CHANGELOG.md: {error}", krate.dir))?;
    let newest = splitlines(&text)
        .into_iter()
        .filter_map(heading)
        .filter(|(version, date)| !date.is_empty() && !version.eq_ignore_ascii_case("unreleased"))
        .max_by(|a, b| compare_versions(&a.0, &b.0));
    Ok(newest.map(|(version, date)| Release { version, date }))
}

/// Registries the crate is published to: crates.io unless `publish = false`,
/// plus npm, PyPI and NuGet when their manifests exist, and the GitHub release's
/// prebuilt archives when it has a CMake package. These are the same files the
/// release workflow publishes from (`scripts/release-crate.py`).
///
/// Empty for a crate that has never been released: a manifest that *could*
/// publish is not a package anyone can install, and a link to it would 404.
pub(crate) fn registries(workspace: &Workspace, krate: &Crate) -> Vec<Registry> {
    let mut out = Vec::new();
    let Some(release) = latest(workspace, krate).ok().flatten() else {
        return out;
    };
    if krate.publish {
        out.push(Registry {
            kind: "crates.io",
            package: krate.name.clone(),
            url: format!("https://crates.io/crates/{}", krate.name),
        });
    }
    let dir = workspace.root.join(&krate.dir);
    if let Some(name) = std::fs::read_to_string(dir.join("npm/package.json"))
        .ok()
        .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
        .and_then(|json| json.get("name")?.as_str().map(str::to_owned))
    {
        out.push(Registry {
            kind: "npm",
            url: format!("https://www.npmjs.com/package/{name}"),
            package: name,
        });
    }
    if let Some(name) = std::fs::read_to_string(dir.join("pyproject.toml"))
        .ok()
        .and_then(|text| project_name(&text))
    {
        out.push(Registry {
            kind: "PyPI",
            url: format!("https://pypi.org/project/{name}/"),
            package: name,
        });
    }
    if let Some(name) = std::fs::read_to_string(dir.join(NUGET_PROJECT))
        .ok()
        .and_then(|text| package_id(&text))
    {
        out.push(Registry {
            kind: "NuGet",
            url: format!("https://www.nuget.org/packages/{name}"),
            package: name,
        });
    }
    // A CMake package ships as prebuilt archives on the crate's GitHub
    // release (release.yml, `native-archives`).
    if let Some(name) = std::fs::read_to_string(dir.join("CMakeLists.txt"))
        .ok()
        .and_then(|text| cmake_project_name(&text))
    {
        out.push(Registry {
            kind: "GitHub release",
            url: format!(
                "https://github.com/openbimrs/ifc/releases/tag/{}-v{}",
                krate.name, release.version
            ),
            package: name,
        });
    }
    out
}

/// The project of a crate shipped as a NuGet package (`openbim-ifc-dotnet`).
pub(crate) const NUGET_PROJECT: &str = "dotnet/OpenBim.Ifc/OpenBim.Ifc.csproj";

/// The `<PackageId>` of a `.csproj`.
fn package_id(text: &str) -> Option<String> {
    let start = text.find("<PackageId>")? + "<PackageId>".len();
    let end = start + text[start..].find("</PackageId>")?;
    let id = text[start..end].trim();
    (!id.is_empty()).then(|| id.to_owned())
}

/// The name in a `CMakeLists.txt`'s `project(<name> ...)` call.
fn cmake_project_name(text: &str) -> Option<String> {
    text.lines().find_map(|line| {
        let args = line.trim().strip_prefix("project(")?;
        let name = args.split(|c: char| c.is_whitespace() || c == ')').next()?;
        (!name.is_empty()).then(|| name.to_owned())
    })
}

/// `name = "…"` in the `[project]` table of a `pyproject.toml`.
fn project_name(text: &str) -> Option<String> {
    let mut in_project = false;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_project = line == "[project]";
            continue;
        }
        if !in_project {
            continue;
        }
        if let Some(value) = line.strip_prefix("name") {
            let value = value.trim_start().strip_prefix('=')?.trim();
            return Some(value.trim_matches('"').to_owned());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cmake_project_name_is_the_first_project_argument() {
        let text =
            "cmake_minimum_required(VERSION 3.21)\nproject(openbim_ifc VERSION 1.0 LANGUAGES C)\n";
        assert_eq!(cmake_project_name(text).as_deref(), Some("openbim_ifc"));
        assert_eq!(
            cmake_project_name("project(solo)\n").as_deref(),
            Some("solo")
        );
        assert_eq!(cmake_project_name("# project(x)\n"), None);
    }

    #[test]
    fn package_id_is_read_from_the_csproj() {
        let text = "<Project>\n  <PropertyGroup>\n    <PackageId>OpenBim.Ifc</PackageId>\n";
        assert_eq!(package_id(text).as_deref(), Some("OpenBim.Ifc"));
        assert_eq!(package_id("<Project />"), None);
    }

    #[test]
    fn pyproject_name_is_read_from_project_table() {
        let text = "[build-system]\nname = \"x\"\n[project]\nname = \"openbim-ifc\"\n";
        assert_eq!(project_name(text).as_deref(), Some("openbim-ifc"));
        assert_eq!(project_name("[tool]\nname = \"y\"\n"), None);
    }
}
