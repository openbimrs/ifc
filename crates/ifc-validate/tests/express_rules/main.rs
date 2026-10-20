//! The EXPRESS evaluator against the normative schemas (#461).
//!
//! Every target here reads the release's `.exp` from the checked-out
//! `references/ifc-spec/`. The schemas are CC BY-ND 4.0 and not committed,
//! so the tests skip without them -- unless `IFC_SPEC_REQUIRED` is set, as
//! `scripts/gate.sh` does, which turns the skip into a failure.

mod corpus;
mod expected;
mod fixtures;
mod inventory;
mod schemas;
mod spot;

use std::path::PathBuf;
use std::sync::OnceLock;

use ifc_schema::{Schema, SchemaVersion};
use ifc_validate::express::ExpressRules;

/// Every bundled release and its normative file.
pub const RELEASES: [(SchemaVersion, &str); 5] = [
    (SchemaVersion::Ifc2x3, "ifc2x3-tc1/IFC2X3_TC1.exp"),
    (SchemaVersion::Ifc4, "ifc4-add2-tc1/IFC4.exp"),
    (SchemaVersion::Ifc4x1, "ifc4x1-final/IFC4x1.exp"),
    (SchemaVersion::Ifc4x2, "ifc4x2-final/IFC4x2.exp"),
    (SchemaVersion::Ifc4x3, "ifc4x3-add2/IFC4X3_ADD2.exp"),
];

fn spec_root() -> Option<PathBuf> {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    [
        "../../references/ifc-spec",
        "../../../../references/ifc-spec",
    ]
    .into_iter()
    .map(|rel| crate_dir.join(rel))
    .find(|path| path.is_dir())
}

/// Whether the schemas are present; panics when they are required but
/// missing.
pub fn spec_available() -> bool {
    let present = spec_root().is_some();
    if !present {
        assert!(
            std::env::var_os("IFC_SPEC_REQUIRED").is_none(),
            "IFC_SPEC_REQUIRED is set but references/ifc-spec was not found"
        );
        eprintln!("skipped: references/ifc-spec not present");
    }
    present
}

/// The bundled tables of `version`.
pub fn schema(version: SchemaVersion) -> &'static Schema {
    ifc_schema::for_version(version).expect("every release is bundled in tests")
}

/// The parsed rules of `version`, once per process.
pub fn rules(version: SchemaVersion) -> &'static ExpressRules {
    static CACHE: [OnceLock<ExpressRules>; 5] = [
        OnceLock::new(),
        OnceLock::new(),
        OnceLock::new(),
        OnceLock::new(),
        OnceLock::new(),
    ];
    let index = RELEASES
        .iter()
        .position(|(v, _)| *v == version)
        .expect("a bundled release");
    CACHE[index].get_or_init(|| {
        let root = spec_root().expect("checked by spec_available");
        let bytes = std::fs::read(root.join(RELEASES[index].1)).expect("the schema file reads");
        ExpressRules::parse_bytes(&bytes, schema(version))
            .unwrap_or_else(|error| panic!("{version:?}: {error}"))
    })
}
