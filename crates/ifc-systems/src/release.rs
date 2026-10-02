//! The release a read binds to.
//!
//! Every accessor in this crate — `systems`, `zones`, `ports`, `flow::of`,
//! `connectivity::build` — must interpret entities against the IFC release
//! the file actually declares, not against a hard-wired one (issue #52), and
//! refuses a file that binds no verified release instead of reading it as
//! IFC4.
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

    fn bind(version: SchemaVersion) -> Result<Self, SchemaResolutionError> {
        let schema =
            for_version(version).map_err(|_| SchemaResolutionError::UnsupportedSchema {
                schema: version.release_id().to_owned(),
            })?;
        Ok(Self { version, schema })
    }
}

/// Releases every reader in this crate is verified against its own table.
///
/// IFC2X3 and IFC4 since #52; IFC4X3 since #215: the zone readers in #194,
/// the system, port, connectivity and flow readers by `tests/ifc4x3.rs`,
/// which round-trips each through STEP text, and by `slots_hold_in_every_verified_release`
/// below, which pins every fixed slot against each table. IFC4X1 and IFC4X2
/// are bundled but unverified here, so they are refused.
const VERIFIED: &[SchemaVersion] = &[
    SchemaVersion::Ifc2x3,
    SchemaVersion::Ifc4,
    SchemaVersion::Ifc4x3,
];

/// Resolve the IFC release a model declares, from its `FILE_SCHEMA` header.
///
/// This is the seam every read in this crate goes through: [`crate::systems`],
/// [`crate::ports`], [`crate::zones`], [`crate::ConnectionGraph::build`],
/// [`crate::ElementRole::of`], [`crate::role_inconsistencies`] and
/// [`crate::spatial_placements`] all refuse exactly what this refuses, with
/// the same error. Nothing is read against a release the file did not
/// declare.
///
/// # Errors
///
/// [`SchemaResolutionError`] when the header names no schema, more than one,
/// or a release this crate has not verified (anything but IFC2X3, IFC4 and
/// IFC4X3), or one this build does not bundle.
pub fn schema_of(model: &Model) -> Result<SchemaVersion, SchemaResolutionError> {
    resolve(model).map(|release| release.version)
}

/// Internal resolution: version tag plus the bundled table to read against.
pub(crate) fn resolve(model: &Model) -> Result<Release, SchemaResolutionError> {
    match model.header().schema.as_slice() {
        [] => Err(SchemaResolutionError::MissingSchema),
        [token] => match SchemaVersion::from_header_token(token) {
            Some(version) if VERIFIED.contains(&version) => Release::bind(version),
            _ => Err(SchemaResolutionError::UnsupportedSchema {
                schema: token.clone(),
            }),
        },
        schemas => Err(SchemaResolutionError::MultipleSchemas {
            schemas: schemas.len(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every fixed slot position these readers use, by entity and attribute,
    /// holds in each verified release's own table. A release whose layout
    /// moved one of them must fail here before it is added to `VERIFIED`.
    #[test]
    fn slots_hold_in_every_verified_release() {
        use crate::connectivity::relation::slot as connects;
        use crate::port::definition::slot as port;
        use crate::system::group::slot as group;
        use crate::system::services::slot as services;
        use crate::zone::spatial_group::slot as spatial;
        let pinned = [
            (
                "IFCRELASSIGNSTOGROUP",
                "RelatedObjects",
                group::ASSIGNS_RELATED,
            ),
            (
                "IFCRELASSIGNSTOGROUP",
                "RelatingGroup",
                group::ASSIGNS_GROUP,
            ),
            ("IFCRELCONNECTSPORTS", "RelatingPort", connects::RELATING),
            ("IFCRELCONNECTSPORTS", "RelatedPort", connects::RELATED),
            (
                "IFCRELCONNECTSPORTS",
                "RealizingElement",
                connects::REALIZING,
            ),
            ("IFCRELNESTS", "RelatingObject", port::NESTS_PARENT),
            ("IFCRELNESTS", "RelatedObjects", port::NESTS_CHILDREN),
            (
                "IFCRELCONNECTSPORTTOELEMENT",
                "RelatingPort",
                port::PORT_TO_ELEMENT_PORT,
            ),
            (
                "IFCRELCONNECTSPORTTOELEMENT",
                "RelatedElement",
                port::PORT_TO_ELEMENT_ELEMENT,
            ),
            ("IFCDISTRIBUTIONPORT", "FlowDirection", port::FLOW_DIRECTION),
            (
                "IFCRELCONTAINEDINSPATIALSTRUCTURE",
                "RelatedElements",
                spatial::RELATED_ELEMENTS,
            ),
            (
                "IFCRELCONTAINEDINSPATIALSTRUCTURE",
                "RelatingStructure",
                spatial::RELATING_STRUCTURE,
            ),
            (
                "IFCRELREFERENCEDINSPATIALSTRUCTURE",
                "RelatedElements",
                spatial::RELATED_ELEMENTS,
            ),
            (
                "IFCRELREFERENCEDINSPATIALSTRUCTURE",
                "RelatingStructure",
                spatial::RELATING_STRUCTURE,
            ),
            (
                "IFCRELSERVICESBUILDINGS",
                "RelatingSystem",
                services::RELATING_SYSTEM,
            ),
            (
                "IFCRELSERVICESBUILDINGS",
                "RelatedBuildings",
                services::RELATED_BUILDINGS,
            ),
            ("IFCSYSTEM", "Name", group::NAME),
            ("IFCDISTRIBUTIONPORT", "Name", port::NAME),
        ];
        let mut checked = 0;
        for &version in VERIFIED {
            let release = Release::bind(version).expect("verified releases are bundled");
            for (entity, attribute, slot) in pinned {
                assert_eq!(
                    release.slot(entity, attribute),
                    Some(slot),
                    "{version:?} {entity}.{attribute}"
                );
                checked += 1;
            }
        }
        assert_eq!(checked, 3 * pinned.len());
    }

    #[test]
    fn unbound_and_unverified_headers_are_refused() {
        let mut model = Model::new();
        assert!(matches!(
            resolve(&model),
            Err(SchemaResolutionError::MissingSchema)
        ));
        for token in ["IFC4X1", "IFC4X2", "IFC9"] {
            model.header_mut().schema = vec![token.to_owned()];
            assert!(
                matches!(resolve(&model), Err(SchemaResolutionError::UnsupportedSchema { ref schema }) if schema == token),
                "{token}"
            );
        }
        model.header_mut().schema = vec!["IFC4".into(), "IFC2X3".into()];
        assert!(matches!(
            resolve(&model),
            Err(SchemaResolutionError::MultipleSchemas { schemas: 2 })
        ));
        model.header_mut().schema = vec!["IFC4X3_ADD2".into()];
        assert_eq!(schema_of(&model), Ok(SchemaVersion::Ifc4x3));
    }
}
