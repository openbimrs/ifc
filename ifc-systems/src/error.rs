//! Why a system query failed.
//!
//! Reading a system means reading relationship entities that may name
//! entities the file never defines. That is a property of real exports, not
//! a programming error, so it is reported rather than panicked on.

use ifc_model::EntityId;
use ifc_schema::SchemaVersion;

/// A system membership the file states but cannot support.
///
/// Anomalies are collected instead of rejected: a file with one broken
/// relationship still has a usable system graph, and refusing the whole
/// model would make the crate useless on real exports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SystemAnomaly {
    /// A relationship names an entity that is not in the file.
    Dangling {
        /// The relationship entity that made the claim.
        relation: EntityId,
        /// The id it named.
        missing: EntityId,
    },
    /// An `IfcZone` member that WR1 does not permit.
    ///
    /// WR1 restricts zone members to `IfcZone`, `IfcSpace` and
    /// `IfcSpatialZone`. Anything else makes the file invalid, so it is
    /// reported and excluded rather than silently listed as zone content.
    ZoneMemberNotSpatial {
        /// The `IfcRelAssignsToGroup` stating it.
        relation: EntityId,
        /// The zone.
        zone: EntityId,
        /// The member WR1 rejects.
        member: EntityId,
        /// Its type, for diagnosis.
        type_name: String,
    },
    /// An element contained by two different spatial structures.
    ///
    /// `ContainedInStructure` is `SET [0:1]`: an element has one home. Two
    /// cannot both be true, so the first by id wins and the conflict is
    /// stated rather than silently resolved.
    ContainedTwice {
        /// The element with two homes.
        element: EntityId,
        /// The structure kept.
        first: EntityId,
        /// The structure rejected.
        second: EntityId,
    },
    /// A port is attached to two different elements.
    ///
    /// `IfcPort.ContainedIn` is `SET [0:1]` in the schema, so this cannot be
    /// expressed by a valid file. It happens when an exporter writes both an
    /// `IfcRelNests` and a legacy `IfcRelConnectsPortToElement` that disagree.
    /// The first attachment in file order is kept so the result stays
    /// deterministic, and the conflict is reported rather than hidden.
    PortAttachedTwice {
        /// The port with two owners.
        port: EntityId,
        /// The element that was kept.
        kept: EntityId,
        /// The element that was rejected.
        rejected: EntityId,
    },
    /// A connection names a port that is not an `IfcPort` subtype.
    ///
    /// `IfcRelConnectsPorts` is typed to `IfcPort` in the schema, so this is a
    /// malformed file rather than a modelling choice.
    NotAPort {
        /// The relationship entity.
        relation: EntityId,
        /// The entity it named as a port.
        entity: EntityId,
        /// That entity's declared type, upper-cased.
        type_name: String,
    },
    /// `IfcRelAssignsToGroup` whose `RelatingGroup` is not a system.
    ///
    /// The relationship is shared with every other kind of group, so a
    /// membership may legitimately point at something this crate does not
    /// model. It is recorded rather than silently dropped.
    NotASystem {
        /// The relationship entity.
        relation: EntityId,
        /// The group it named.
        group: EntityId,
        /// The group's declared type, upper-cased.
        type_name: String,
    },
}

/// Why a model's declared schema release could not be resolved.
///
/// Every read path in this crate binds to the IFC release the file's
/// `FILE_SCHEMA` header declares (see [`crate::schema_of`]) rather than
/// assuming IFC4. A file that does not name a release this crate has
/// verified semantics for is refused, not silently read under the wrong
/// table: guessing IFC4 for an IFC2X3 file mis-classifies `IfcZone` as a
/// system and mis-reads `IfcElectricalCircuit` as not one (issue #52).
///
/// `#[non_exhaustive]`: new refusal reasons (e.g. a newly-verified release
/// gaining support) must be addable without breaking callers matching on
/// this type.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SchemaResolutionError {
    /// `FILE_SCHEMA` names no schema at all.
    MissingSchema,
    /// `FILE_SCHEMA` names more than one schema; this crate reads only
    /// single-schema files.
    MultipleSchemas {
        /// How many schema tokens the header carried.
        schemas: usize,
    },
    /// `FILE_SCHEMA` names a release this crate does not read against.
    ///
    /// Only IFC2X3 and IFC4 are resolved today. IFC4X3 is bundled in
    /// `ifc-schema` but its distribution-system semantics have not been
    /// verified for this crate, so it is refused rather than defaulted to
    /// IFC4 -- an IFC4X3 file assumed to be IFC4 would misread
    /// `IfcBuiltSystem` and related IFC4X3-only entities.
    UnsupportedSchema {
        /// The header token as written, e.g. `"IFC4X3_ADD2"`.
        schema: String,
    },
}

impl std::fmt::Display for SchemaResolutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingSchema => write!(f, "FILE_SCHEMA declares no schema"),
            Self::MultipleSchemas { schemas } => {
                write!(
                    f,
                    "FILE_SCHEMA declares {schemas} schemas, expected exactly one"
                )
            }
            Self::UnsupportedSchema { schema } => {
                write!(
                    f,
                    "schema {schema:?} is not resolved by ifc-systems (only IFC2X3 and IFC4 are)"
                )
            }
        }
    }
}

impl std::error::Error for SchemaResolutionError {}

/// An accessor that reads an attribute the declared release does not
/// define for the entity's type.
///
/// IFC2X3's `IfcZone` has no `LongName` slot; asking for one under that
/// release is a different fact than the file having authored an empty
/// value, and conflating the two (`None`) would make "not in this schema"
/// indistinguishable from "authored empty" (issue #52).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct NotInSchema {
    /// The entity whose type lacks the attribute.
    pub entity: EntityId,
    /// The schema release that was checked.
    pub schema: SchemaVersion,
}

impl std::fmt::Display for NotInSchema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "attribute not declared for entity {:?} under {:?}",
            self.entity, self.schema
        )
    }
}

impl std::error::Error for NotInSchema {}

/// Why a per-attribute accessor bound to the model's declared release could
/// not produce a value.
///
/// Reading an attribute that varies by release (e.g. `IfcZone.LongName`,
/// absent in IFC2X3) needs the model's release resolved first. Either step
/// can fail: the model's own `FILE_SCHEMA` may not resolve at all
/// ([`SchemaResolutionError`], see [`crate::schema_of`]), or it may resolve
/// to a release that simply does not declare the attribute
/// ([`NotInSchema`]). Both are reported through this one error so a caller
/// has a single type to match on.
///
/// `#[non_exhaustive]`: new attribute-accessor call sites reuse this type,
/// and adding one must not be a breaking change for existing matches.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SchemaGap {
    /// The model's declared schema could not be resolved.
    Schema(SchemaResolutionError),
    /// The resolved release does not declare this attribute for this entity.
    NotInSchema(NotInSchema),
}

impl From<SchemaResolutionError> for SchemaGap {
    fn from(error: SchemaResolutionError) -> Self {
        Self::Schema(error)
    }
}

impl From<NotInSchema> for SchemaGap {
    fn from(error: NotInSchema) -> Self {
        Self::NotInSchema(error)
    }
}

impl std::fmt::Display for SchemaGap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Schema(error) => write!(f, "{error}"),
            Self::NotInSchema(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for SchemaGap {}
