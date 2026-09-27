//! Repository gate: every `.ifc` fixture has a checked provenance entry.
//!
//! `test/fixtures/PROVENANCE.tsv` records, per fixture, where the bytes came
//! from (a commit-pinned upstream URL, the fixture it was edited from, the
//! generator, or the commit that hand-authored it), the licence, and for
//! upstream copies whether the file is still byte-identical to upstream.
//! Prose and naming conventions drift; this test makes an unrecorded fixture,
//! a stale entry, or a false "identical" claim a build failure.
//!
//! It lives in `ifc-model` beside the other repository gates
//! (`no_monolithic_files`, `progressive_context`) because it reads files, not
//! IFC semantics: no codec, schema or domain crate is involved.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

const MANIFEST: &str = "test/fixtures/PROVENANCE.tsv";
const HEADER: [&str; 7] = [
    "path",
    "origin",
    "source",
    "licence",
    "upstream_sha256",
    "identical",
    "note",
];

/// Licences a fixture may carry. Adding one is a deliberate licensing
/// decision, so it is a code change here rather than a free-text column.
const LICENCES: &[&str] = &["AGPL-3.0-or-later", "LGPL-3.0-or-later", "MPL-2.0"];

/// Fewest rows the manifest may hold; a parse that silently yields nothing
/// must not pass as "every fixture is covered".
const MIN_ROWS: usize = 40;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("ifc-model sits in the repository root")
        .to_path_buf()
}

/// Every `.ifc` under `dir`, as a `/`-separated path relative to `base`.
fn ifc_files(base: &Path, dir: &Path, out: &mut BTreeSet<String>) {
    let entries = std::fs::read_dir(dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display()));
    for entry in entries {
        let path = entry.expect("directory entry").path();
        if path.is_dir() {
            ifc_files(base, &path, out);
        } else if path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("ifc"))
        {
            let relative = path.strip_prefix(base).expect("under the fixture root");
            let parts: Vec<_> = relative.iter().map(|p| p.to_string_lossy()).collect();
            out.insert(parts.join("/"));
        }
    }
}

/// One manifest row, by column name.
struct Row<'a> {
    line: usize,
    cells: BTreeMap<&'static str, &'a str>,
}

impl Row<'_> {
    fn get(&self, column: &str) -> &str {
        self.cells[column]
    }
}

fn parse(text: &str) -> Vec<Row<'_>> {
    let mut lines = text
        .lines()
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty() && !line.starts_with('#'));
    let (_, header) = lines.next().expect("manifest has a header row");
    assert_eq!(
        header.split('\t').collect::<Vec<_>>(),
        HEADER,
        "{MANIFEST}: header row"
    );
    lines
        .map(|(index, line)| {
            let fields: Vec<&str> = line.split('\t').collect();
            assert_eq!(
                fields.len(),
                HEADER.len(),
                "{MANIFEST}:{}: expected {} tab-separated columns",
                index + 1,
                HEADER.len()
            );
            Row {
                line: index + 1,
                cells: HEADER.iter().copied().zip(fields).collect(),
            }
        })
        .collect()
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn is_hex(text: &str, len: std::ops::RangeInclusive<usize>) -> bool {
    len.contains(&text.len()) && text.bytes().all(|b| b.is_ascii_hexdigit())
}

/// A pinned upstream URL: `https://github.com/<owner>/<repo>/blob/<sha>/...`.
fn pinned_url(source: &str) -> bool {
    let Some(rest) = source.strip_prefix("https://github.com/") else {
        return false;
    };
    let parts: Vec<&str> = rest.splitn(5, '/').collect();
    parts.len() == 5 && parts[2] == "blob" && is_hex(parts[3], 40..=40) && !parts[4].is_empty()
}

/// Every problem with one row; empty when the row is sound.
fn check_row(root: &Path, fixtures: &Path, row: &Row<'_>, listed: &BTreeSet<&str>) -> Vec<String> {
    let mut problems = Vec::new();
    let path = row.get("path");
    let file = fixtures.join(path);
    if !file.is_file() {
        problems.push(format!("names a missing file {path}"));
    }
    if !LICENCES.contains(&row.get("licence")) {
        problems.push(format!("unknown licence {:?}", row.get("licence")));
    }
    if row.get("note").trim().is_empty() {
        problems.push("has an empty note".into());
    }
    let source = row.get("source");
    let (sha, identical) = (row.get("upstream_sha256"), row.get("identical"));
    match row.get("origin") {
        "upstream" => {
            if !pinned_url(source) {
                problems.push(format!("upstream source is not commit-pinned: {source}"));
            }
            if !is_hex(sha, 64..=64) {
                problems.push(format!("upstream_sha256 is not a SHA-256: {sha}"));
            }
            let claimed = match identical {
                "yes" => Some(true),
                "no" => Some(false),
                other => {
                    problems.push(format!("identical must be yes or no, not {other:?}"));
                    None
                }
            };
            if let (Some(claimed), Ok(bytes)) = (claimed, std::fs::read(&file)) {
                let actual = sha256_hex(&bytes) == sha;
                if actual != claimed {
                    problems.push(format!(
                        "claims identical={identical}, but the local bytes {} upstream_sha256",
                        if actual { "match" } else { "differ from" }
                    ));
                }
            }
        }
        origin @ ("derived" | "generated" | "hand-authored") => {
            if sha != "-" || identical != "-" {
                problems.push(format!(
                    "{origin} rows leave upstream_sha256 and identical as -"
                ));
            }
            let source_ok = match origin {
                "derived" => listed.contains(source) && source != path,
                "generated" => root.join(source).is_file(),
                _ => source
                    .strip_prefix("commit:")
                    .is_some_and(|c| is_hex(c, 7..=40)),
            };
            if !source_ok {
                problems.push(format!("{origin} source is not valid: {source}"));
            }
        }
        other => problems.push(format!("unknown origin {other:?}")),
    }
    problems
        .into_iter()
        .map(|p| format!("{MANIFEST}:{}: {path}: {p}", row.line))
        .collect()
}

/// Every problem with a manifest `text` against the fixtures on disk.
fn audit(root: &Path, text: &str) -> Vec<String> {
    let fixtures = root.join("test/fixtures");
    let rows = parse(text);
    let mut on_disk = BTreeSet::new();
    ifc_files(&fixtures, &fixtures, &mut on_disk);

    let mut problems = Vec::new();
    let mut listed = BTreeSet::new();
    for row in &rows {
        if !listed.insert(row.get("path")) {
            problems.push(format!(
                "{MANIFEST}:{}: {} listed twice",
                row.line,
                row.get("path")
            ));
        }
    }
    for row in &rows {
        problems.extend(check_row(root, &fixtures, row, &listed));
    }
    for missing in on_disk.iter().filter(|f| !listed.contains(f.as_str())) {
        problems.push(format!("{missing} has no entry in {MANIFEST}"));
    }
    problems
}

fn manifest(root: &Path) -> String {
    std::fs::read_to_string(root.join(MANIFEST)).unwrap_or_else(|e| panic!("{MANIFEST}: {e}"))
}

#[test]
fn every_fixture_has_exactly_one_sound_provenance_entry() {
    let root = repo_root();
    let text = manifest(&root);
    let rows = parse(&text).len();
    assert!(rows >= MIN_ROWS, "{MANIFEST}: only {rows} rows parsed");

    let fixtures = root.join("test/fixtures");
    let mut on_disk = BTreeSet::new();
    ifc_files(&fixtures, &fixtures, &mut on_disk);
    assert!(
        on_disk.len() >= MIN_ROWS,
        "only {} fixtures found; is the walk looking in the right place?",
        on_disk.len()
    );

    let problems = audit(&root, &text);
    assert!(problems.is_empty(), "\n{}", problems.join("\n"));
}

/// Removing one row from the real manifest is caught by name.
#[test]
fn a_fixture_without_an_entry_is_reported() {
    let root = repo_root();
    let text = manifest(&root);
    let dropped = "hostile/string_literals.ifc";
    let without: String = text
        .lines()
        .filter(|line| !line.starts_with(dropped))
        .map(|line| format!("{line}\n"))
        .collect();
    assert_ne!(without.len(), text.len(), "the row to drop exists");
    let problems = audit(&root, &without);
    assert_eq!(
        problems,
        [format!("{dropped} has no entry in {MANIFEST}")],
        "exactly the dropped fixture is reported"
    );
}

fn row<'a>(fields: [&'a str; 7]) -> Row<'a> {
    Row {
        line: 1,
        cells: HEADER.iter().copied().zip(fields).collect(),
    }
}

/// The row checks refuse what they must; a gate that accepts everything
/// would pass the tests above just as quietly.
#[test]
fn the_row_checks_refuse_bad_rows() {
    let root = repo_root();
    let fixtures = root.join("test/fixtures");
    let path = "hostile/string_literals.ifc";
    let listed: BTreeSet<&str> = [path].into();
    let problems = |fields: [&str; 7]| check_row(&root, &fixtures, &row(fields), &listed);

    let hand = [
        path,
        "hand-authored",
        "commit:9bc4a5a",
        "AGPL-3.0-or-later",
        "-",
        "-",
        "Hand-written.",
    ];
    assert_eq!(problems(hand), Vec::<String>::new());
    let edits: [(usize, &str, &str); 11] = [
        (0, "hostile/absent.ifc", ""),
        (1, "found-on-a-usb-stick", ""),
        (2, "9bc4a5a", ""),
        (2, "commit:not-a-sha", ""),
        (3, "WTFPL", ""),
        (4, "abc", ""),
        (5, "yes", ""),
        (6, " ", ""),
        (1, "generated", "tools/no_such_generator.py"),
        (1, "derived", path),
        (1, "derived", "hostile/unlisted.ifc"),
    ];
    for (slot, value, source) in edits {
        let mut fields = hand;
        fields[slot] = value;
        if !source.is_empty() {
            fields[2] = source;
        }
        assert!(!problems(fields).is_empty(), "accepted {fields:?}");
    }

    // An upstream row: pinned to a commit, and honest about identity in
    // both directions.
    let real = sha256_hex(&std::fs::read(fixtures.join(path)).expect("fixture"));
    let other = "0".repeat(64);
    let pinned = "https://github.com/o/r/blob/0123456789012345678901234567890123456789/x.ifc";
    let upstream = |source: &str, sha: &str, identical: &str| {
        problems([
            path, "upstream", source, "MPL-2.0", sha, identical, "Copied.",
        ])
        .is_empty()
    };
    assert!(upstream(pinned, &real, "yes"));
    assert!(upstream(pinned, &other, "no"));
    assert!(!upstream(pinned, &real, "no"));
    assert!(!upstream(pinned, &other, "yes"));
    assert!(!upstream(pinned, &real, "maybe"));
    assert!(!upstream(pinned, "abc", "no"), "a non-SHA-256 is refused");
    for unpinned in [
        "https://github.com/o/r/blob/main/x.ifc",
        "https://github.com/o/r/tree/0123456789012345678901234567890123456789/x.ifc",
        "https://example.org/o/r/blob/0123456789012345678901234567890123456789/x.ifc",
        "https://github.com/o/r/blob/0123456789012345678901234567890123456789/",
    ] {
        assert!(!upstream(unpinned, &real, "yes"), "accepted {unpinned}");
    }
}
