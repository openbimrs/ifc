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

    /// Position of `attribute` on `entity` in this release, by name; `None`
    /// when the release does not declare it.
    ///
    /// Used to tell "the file left this attribute empty" apart from "this
    /// release does not have this attribute" (e.g. IFC2X3 `IfcZone` has no
    /// `LongName`).
    pub(crate) fn slot(self, entity: &str, attribute: &str) -> Option<usize> {
        self.schema
            .attribute_names(entity)
            .iter()
            .position(|name| name.eq_ignore_ascii_case(attribute))
    }

    fn bind(version: SchemaVersion) -> Self {
        Self {
            version,
            schema: for_version(version).expect("every SchemaVersion has a bundled table"),
        }
    }
}

/// Releases whose zone semantics are verified against their own tables:
/// `IfcZone`, `IfcRelAssignsToGroup` and the WR1 member types (#194).
const ZONE_RELEASES: &[SchemaVersion] = &[
    SchemaVersion::Ifc2x3,
    SchemaVersion::Ifc4,
    SchemaVersion::Ifc4x3,
];

/// Releases verified for the system, port, flow and connectivity readers.
/// IFC4X3's distribution semantics (`IfcBuiltSystem`, the IFC4X3
/// distribution-system enumeration) are not verified yet.
const SYSTEM_RELEASES: &[SchemaVersion] = &[SchemaVersion::Ifc2x3, SchemaVersion::Ifc4];

/// Resolve the IFC release a model declares, from its `FILE_SCHEMA` header.
///
/// This is the public seam issue #52 asks every read path to go through:
/// callers who want to know (or assert) which release a model will be read
/// under -- without pulling in `ifc-schema` themselves -- can call this
/// directly. It refuses a file with no schema, more than one, or a release
/// this crate has not verified for its system, port and flow readers
/// (including IFC4X3). The zone readers ([`crate::zones`],
/// [`crate::try_zones`], [`crate::long_name_of`]) are verified for IFC4X3
/// and bind its table (#194); this function still refuses it.
///
/// The crate's bulk accessors (`systems`, `zones`, `ports`,
/// `ElementRole::of`, `ConnectionGraph::build`, ...) do NOT surface that
/// refusal: their signatures have no error slot, so where this function
/// would refuse they read the model under the IFC4 table instead. A caller
/// who needs the refusal calls this first and acts on its `Err`.
///
/// # Errors
///
/// Returns [`SchemaResolutionError`] when the header names zero or more
/// than one schema, or names a schema other than IFC2X3 or IFC4.
pub fn schema_of(model: &Model) -> Result<SchemaVersion, SchemaResolutionError> {
    resolve(model).map(|release| release.version)
}

/// Internal resolution: version tag plus the bundled table to read against.
///
/// IFC4X3 is bundled in ifc-schema, but the system, port, flow and
/// connectivity reads have only been verified against IFC2X3 and IFC4
/// semantics (#52 scope); it stays refused here until that verification
/// happens. Zones resolve through [`resolve_zones`] instead.
pub(crate) fn resolve(model: &Model) -> Result<Release, SchemaResolutionError> {
    resolve_among(model, SYSTEM_RELEASES)
}

/// [`resolve`] for the zone readers, which are verified for IFC4X3 too.
pub(crate) fn resolve_zones(model: &Model) -> Result<Release, SchemaResolutionError> {
    resolve_among(model, ZONE_RELEASES)
}

fn resolve_among(
    model: &Model,
    verified: &[SchemaVersion],
) -> Result<Release, SchemaResolutionError> {
    match model.header().schema.as_slice() {
        [] => Err(SchemaResolutionError::MissingSchema),
        [token] => match SchemaVersion::from_header_token(token) {
            Some(version) if verified.contains(&version) => Ok(Release::bind(version)),
            _ => Err(SchemaResolutionError::UnsupportedSchema {
                schema: token.clone(),
            }),
        },
        schemas => Err(SchemaResolutionError::MultipleSchemas {
            schemas: schemas.len(),
        }),
    }
}

/// The IFC4 table, which the bulk readers fall back to.
pub(crate) fn ifc4() -> Release {
    Release::bind(SchemaVersion::Ifc4)
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
    resolve(model).unwrap_or_else(|_| ifc4())
}

/// [`resolve_or_ifc4`] for `zones()`: an IFC4X3 header binds the IFC4X3
/// table; a header binding nothing falls back to IFC4 as before.
pub(crate) fn resolve_zones_or_ifc4(model: &Model) -> Release {
    resolve_zones(model).unwrap_or_else(|_| ifc4())
}
