//! Header well-formedness and the schema a file declares.
//!
//! # What survives to here, and what does not
//!
//! [`ifc_model::header::Header`] is normalized: it holds a description list,
//! an author list, a schema token list. A STEP `FILE_NAME` entry with too
//! many parameters, or with an integer where a string belongs, is a *syntax*
//! defect -- the codec either recovers from it or fails, and by the time a
//! `Model` exists the original arity is gone.
//!
//! So this module deliberately does not attempt to re-derive header arity
//! errors. It checks what the normalized header can still answer:
//!
//! - is a schema declared at all, and is it one this build can validate?
//!   A release known by name but published without an EXPRESS schema
//!   (`IFC4X3_TC1`, `IFC4X3_ADD1`; [`ifc_schema::UnpublishedRelease`]) is
//!   reported as such, `header.schema.unpublished`, not as unknown (#467).
//! - is the implementation level the Part 21 value?
//!
//! A caller that needs header syntax diagnostics wants them from the codec's
//! own diagnostic channel, which sees the file. Claiming to check them here
//! would report "clean" for a file this crate never actually inspected.

use ifc_model::Model;
use ifc_schema::{SchemaVersion, UnpublishedRelease};

use crate::report::{Finding, Path, Report};

/// Check the declared schema and implementation level.
pub fn check(model: &Model, report: &mut Report) {
    let header = model.header();
    match header.schema_token() {
        None => report.push(Finding::error(
            "header.schema.missing",
            Path::File,
            "FILE_SCHEMA declares no schema; the file cannot be validated \
             against anything",
        )),
        Some(token) if SchemaVersion::from_header_token(token).is_some() => {}
        Some(token) => match UnpublishedRelease::from_header_token(token) {
            // Tables passed by the caller (`validate`) are the caller's
            // explicit choice; this finding keeps the report from reading
            // as a check against the declared release.
            Some(release) => report.push(Finding::warning(
                "header.schema.unpublished",
                Path::File,
                format!(
                    "schema {token:?} is IFC {} ({}), a release buildingSMART \
                     publishes no EXPRESS schema for; it is not checked against \
                     its own tables, and {} is not assumed to match it",
                    release.version_number(),
                    release.identifier(),
                    release.superseded_by().release_id()
                ),
            )),
            None => report.push(Finding::warning(
                "header.schema.unknown",
                Path::File,
                format!(
                    "schema {token:?} is not one this build recognizes; \
                     structural checks will be skipped"
                ),
            )),
        },
    }

    // Part 21 fixes this to "2;1". A different value is not fatal -- files
    // carrying it parse fine -- but it signals a producer that is not
    // following the standard, which usually means other things are off too.
    let level = &header.implementation_level;
    if !level.is_empty() && level != "2;1" {
        report.push(Finding::warning(
            "header.implementation_level",
            Path::File,
            format!("implementation level is {level:?}; ISO 10303-21 specifies \"2;1\""),
        ));
    }
}

/// The schema version a model declares, if this build recognizes it.
#[must_use]
pub fn declared_version(model: &Model) -> Option<SchemaVersion> {
    model
        .header()
        .schema_token()
        .and_then(SchemaVersion::from_header_token)
}
