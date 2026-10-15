//! The committed fixture corpus the dispatch tests walk.
//!
//! Split out of the parent suite to keep each file under the repo line cap;
//! the assertions stay in the tests that consume these.

use std::path::{Path, PathBuf};

use ifc_geometry::GeometryError;
use ifc_model::Model;

pub fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../test/fixtures")
}

pub fn collect_ifc(dir: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_ifc(&path, files);
        } else if path.extension().and_then(|e| e.to_str()) == Some("ifc") {
            files.push(path);
        }
    }
}

/// The one committed collapsed loop, refused on purpose (#46, #47).
///
/// `synthetic-coverage/meshing_coverage.ifc` carries a faceted brep with one
/// `(A, A, B, B)` sliver face so `tests/meshing_coverage.rs` can pin the
/// default refusal. That refusal is `Degenerate`, not `Unsupported`, and it is
/// correct. Excused only for that file, only as `Degenerate`, and only when it
/// names an `IfcPolyLoop`: a degenerate report anywhere else still fails.
pub fn is_pinned_collapsed_loop(path: &Path, model: &Model, error: &GeometryError) -> bool {
    let named = error.entity().and_then(|id| model.get(id));
    path.ends_with("synthetic-coverage/meshing_coverage.ifc")
        && matches!(error, GeometryError::Degenerate { .. })
        && named.is_some_and(|entity| entity.type_name.eq_ignore_ascii_case("IFCPOLYLOOP"))
}

/// Whether `type_name` is a [`PARTIAL`] family with a refused variant.
///
/// [`PARTIAL`]: ifc_geometry::lower::dispatch::PARTIAL
pub fn declares_refused_variant(type_name: &str) -> bool {
    use ifc_geometry::lower::dispatch::{Support, PARTIAL};
    PARTIAL
        .iter()
        .any(|v| v.support == Support::Refused && v.family.eq_ignore_ascii_case(type_name))
}

const DISPOSITIONS: &str = include_str!("../../data/ifc4-representation-item-dispositions.tsv");

pub fn disposition_rows() -> Vec<[&'static str; 4]> {
    DISPOSITIONS
        .lines()
        .skip(1)
        .map(|line| {
            let fields: Vec<_> = line.split('\t').collect();
            assert_eq!(fields.len(), 4, "malformed disposition row: {line}");
            [fields[0], fields[1], fields[2], fields[3]]
        })
        .collect()
}

/// Entities the disposition ledger classifies as a typed refusal.
///
/// Read from the TSV rather than hard-coded so the escape hatch below can
/// never admit a family the ledger does not actually
/// classify that way.
pub fn typed_refusals() -> Vec<String> {
    disposition_rows()
        .iter()
        .filter(|row| row[1] == "typed-refusal")
        .map(|row| row[0].to_owned())
        .collect()
}
