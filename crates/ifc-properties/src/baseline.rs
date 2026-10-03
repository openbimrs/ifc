//! The table a permissive read falls back to.
//!
//! Permissive reads predate per-release builds and read through the IFC4
//! ADD2 TC1 table whatever the file declares (or when it declares a release
//! the build does not bundle). A build that leaves IFC4 out (#306) reads
//! through the newest release it bundles instead; the crate's compile guard
//! guarantees there is one.

use ifc_schema::{for_version, Schema, SchemaVersion};

/// The order a permissive read tries the bundled tables in: IFC4 first, as
/// before #306, then the newest release.
const BASELINE: [SchemaVersion; 5] = [
    SchemaVersion::Ifc4,
    SchemaVersion::Ifc4x3,
    SchemaVersion::Ifc4x2,
    SchemaVersion::Ifc4x1,
    SchemaVersion::Ifc2x3,
];

/// `preferred` when this build bundles it, otherwise the baseline table.
pub(crate) fn table(preferred: Option<SchemaVersion>) -> (SchemaVersion, &'static Schema) {
    preferred
        .into_iter()
        .chain(BASELINE)
        .find_map(|version| for_version(version).ok().map(|schema| (version, schema)))
        // The compile guard in lib.rs refuses a build bundling no release.
        .expect("ifc-properties bundles at least one release")
}
