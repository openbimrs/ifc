//! Running the built `openbim-ifc` binary over the fixture corpus.

#![allow(dead_code)] // each test file uses part of this

use std::path::{Path, PathBuf};
use std::process::Command;

/// What one run printed and how it ended.
pub struct Run {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

impl Run {
    /// Stdout parsed as JSON.
    pub fn json(&self) -> serde_json::Value {
        serde_json::from_str(&self.stdout)
            .unwrap_or_else(|error| panic!("stdout is not JSON ({error}):\n{}", self.stdout))
    }
}

/// Run the binary with `args` from the repository root, so paths in the
/// output are the relative ones the arguments name.
pub fn run<S: AsRef<std::ffi::OsStr>>(args: &[S]) -> Run {
    let output = Command::new(env!("CARGO_BIN_EXE_openbim-ifc"))
        .args(args)
        .current_dir(root())
        // The gate's authored-coverage run sets this for test processes;
        // the binary only reads and writes files, and its codec loads must
        // not count as coverage of the workspace's own tests.
        .env_remove("AUTHORED_DUMP")
        .output()
        .expect("the binary runs");
    Run {
        code: output.status.code().expect("exited, not signalled"),
        stdout: String::from_utf8(output.stdout).expect("stdout is UTF-8"),
        stderr: String::from_utf8(output.stderr).expect("stderr is UTF-8"),
    }
}

/// The repository root.
pub fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repository root")
}

/// A fixture path relative to the repository root.
pub fn fixture(relative: &str) -> String {
    let path = format!("test/fixtures/{relative}");
    assert!(root().join(&path).is_file(), "{path} is missing");
    path
}

/// Every `.ifc` fixture, relative to the repository root, sorted.
pub fn corpus() -> Vec<String> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        for entry in std::fs::read_dir(dir).expect("fixture directory") {
            let path = entry.expect("entry").path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().is_some_and(|extension| extension == "ifc") {
                out.push(path);
            }
        }
    }
    let root = root();
    let mut found = Vec::new();
    walk(&root.join("test/fixtures"), &mut found);
    let mut relative: Vec<String> = found
        .iter()
        .map(|path| {
            path.strip_prefix(&root)
                .expect("under the root")
                .to_string_lossy()
                .replace('\\', "/")
        })
        .collect();
    relative.sort();
    // A layout change that hides the corpus must fail, not pass vacuously.
    assert!(relative.len() >= 40, "found {} fixtures", relative.len());
    relative
}

/// A scratch directory for one test, inside the build's temporary
/// directory, emptied first.
pub fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("openbim-ifc-cli")
        .join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch directory");
    dir
}
