//! `ifc-schema` — the IFC schema **as data**, not as 2,500 generated structs.
//!
//! # The decision
//!
//! IfcOpenShell generates a C++ class per IFC entity per schema version. That
//! is a very large amount of code to compile, and it multiplies by the number
//! of supported schemas.
//!
//! Evidence from the schemas in `references/ifc-spec/`:
//!
//! | Schema | Entities |
//! |---|---|
//! | IFC2x3 TC1 | 653 |
//! | IFC4 ADD2 TC1 | 776 |
//! | IFC4x3 ADD2 | 876 |
//!
//! Worse, names are *not* stable across versions: IFC4x3 renames
//! `IfcBuildingElement` to `IfcBuiltElement` and removes `IfcProxy`, the whole
//! `*StandardCase` family, `IfcDoorStyle` and `IfcWindowStyle`. Generated
//! per-version types would triple the API surface and force every consumer to
//! choose a version at compile time.
//!
//! We instead treat the schema as a **lookup table** built from the official
//! EXPRESS files. One code path serves every schema version; supporting a new
//! one is data, not a release.
//!
//! # Module map
//!
//! | Module | Role |
//! |---|---|
//! | [`version`] | which schema a file declares |
//! | [`entity`] | entity table: name, supertype, attribute slots |
//! | [`attribute`] | attribute descriptors and their declared types |
//! | [`types`] | defined types, enums, and selects |
//! | [`inheritance`] | supertype chain walking and subtype tests |
//! | [`registry`] | the assembled, queryable schema |
//! | [`express`] | parser for the official `.exp` files |
//!
//! # Status
//!
//! Scaffold. [`version`] detection is implemented and tested; the rest is
//! Stage 1 in `docs/ROADMAP.md`.

pub mod attribute;
pub mod entity;
pub mod express;
pub mod inheritance;
pub mod registry;
pub mod types;
pub mod version;

pub use version::SchemaVersion;
