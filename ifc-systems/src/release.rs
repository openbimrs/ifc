//! The release a read binds to.
//!
//! Every accessor in this crate — `systems`, `zones`, `ports`, `flow::of`,
//! `connectivity::build` — must interpret entities against the IFC release
//! the file actually declares, not against a hard-wired one (issue #52).
//! `IfcZone` is a subtype of `IfcSystem` in IFC4 but of `IfcGroup` in
//! IFC2X3; reading an IFC2X3 file under the IFC4 table therefore
//! misclassifies zones as systems and vice versa for `IfcElectricalCircuit`.
//!
//! This module resolves that release once per read and hands out a small
//! [`Release`] carrying both the version tag (for error reporting) and the
//! bundled [`Schema`] table (for `is_a`/`attributes` lookups), mirroring the
//! pattern `ifc-properties::exact::release` established for issue #48.

use ifc_model::Model;
use ifc_schema::{for_version, Schema, SchemaVersion};

use crate::error::SchemaResolutionError;

/// The release a read runs against: its version tag and bundled table.
///
/// Crate-private: callers get a [`SchemaVersion`] from [`schema_of`] and
/// the crate's public accessors resolve their own `Release` internally, so
/// this type never needs to appear in a public signature.
#[derive(Clone, Copy)]
pub(crate) struct Release {
    pub(crate) version: SchemaVersion,
    pub(crate) schema: &'static Schema,
}

impl Release {
    /// Whether `candidate` (a declared entity type name) is `ancestor` or a
    /// subtype of it, under this release's table.
    pub(crate) fn is_a(self, candidate: &str, ancestor: &str) -> bool {
        self.schema.is_a(candidate, ancestor)
    }

    /// Whether attribute `slot` of `entity` is declared under this release
    /// at all -- i.e. the entity's arity in this schema covers that index.
    ///
    /// Used to tell "the file left this attribute empty" apart from "this
    /// release does not have this attribute" (e.g. IFC2X3 `IfcZone` has no
    /// `LongName`, and IFC2X3 `IfcDistributionPort` has no `PredefinedType`
    /// or `SystemType`).
    pub(crate) fn has_slot(self, entity: &str, slot: usize) -> bool {
        self.schema.attributes(entity).len() > slot
    }
}

/// Resolve the IFC release a model declares, from its `FILE_SCHEMA` header.
///
/// This is the public seam issue #52 asks every read path to go through:
/// callers who want to know (or assert) which release a model will be read
/// under -- without pulling in `ifc-schema` themselves -- can call this
/// directly. The crate's own accessors (`systems`, `zones`, `ports`,
/// `ElementRole::of`, `ConnectionGraph::build`, ...) call it internally and
/// surface the same refusal behaviour: a file with no schema, more than
/// one, or a release this crate has not verified (including IFC4X3) is
/// refused rather than silently read as IFC4.
///
/// # Errors
///
/// Returns [`SchemaResolutionError`] when the header names zero or more
/// than one schema, or names a schema other than IFC2X3 or IFC4.
pub fn schema_of(model: &Model) -> Result<SchemaVersion, SchemaResolutionError> {
    resolve(model).map(|release| release.version)
}

/// Internal resolution: version tag plus the bundled table to read against.
pub(crate) fn resolve(model: &Model) -> Result<Release, SchemaResolutionError> {
    match model.header().schema.as_slice() {
        [] => Err(SchemaResolutionError::MissingSchema),
        [token] => match SchemaVersion::from_header_token(token) {
            // IFC4X3 is bundled in ifc-schema, but this crate's reads have
            // only been verified against IFC2X3 and IFC4 semantics (#52
            // scope); it stays refused until that verification happens.
            Some(version @ (SchemaVersion::Ifc2x3 | SchemaVersion::Ifc4)) => Ok(Release {
                version,
                schema: for_version(version).expect("IFC2X3 and IFC4 are bundled"),
            }),
            _ => Err(SchemaResolutionError::UnsupportedSchema {
                schema: token.clone(),
            }),
        },
        schemas => Err(SchemaResolutionError::MultipleSchemas {
            schemas: schemas.len(),
        }),
    }
}

/// Resolve the declared release for the crate's bulk accessors
/// (`systems`, `zones`, `ports`, `ElementRole::of`, `ConnectionGraph::build`).
///
/// These functions predate #52 and return `(Vec<_>, Vec<SystemAnomaly>)`,
/// not a `Result`: there is no return-type slot to carry a hard refusal
/// without a breaking signature change. So when [`resolve`] cannot bind a
/// release -- the header names no schema, several, or one this crate has
/// not verified (including IFC4X3) -- they fall back to the IFC4 table,
/// exactly as every one of them was hard-wired to do before this change.
/// This keeps every existing caller's behaviour identical for models that
/// never carried a `FILE_SCHEMA` header at all (every hand-built fixture in
/// this crate's own test suite included) while a file that DOES declare
/// IFC2X3 is now read correctly under IFC2X3 semantics.
///
/// A caller who needs a hard refusal instead of this fallback should call
/// [`schema_of`] first and act on its `Err`.
pub(crate) fn resolve_or_ifc4(model: &Model) -> Release {
    resolve(model).unwrap_or_else(|_| Release {
        version: SchemaVersion::Ifc4,
        schema: for_version(SchemaVersion::Ifc4).expect("IFC4 is bundled"),
    })
}
