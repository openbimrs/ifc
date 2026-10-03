//! Subtype resolution without loading the EXPRESS schema at runtime.
//!
//! The tables encode two releases, listed in [`VERIFIED_SCHEMA_VERSIONS`]:
//! IFC4 ADD2 TC1, the base ([`TABLE_SCHEMA_VERSION`]), and IFC4X3 ADD2 as a
//! delta over it. [`is_a_in`] and [`supertypes_of_in`] answer for one named
//! release; ask [`tables_are_verified_for`] before trusting them for another.
//!
//! # Why this table exists
//!
//! EXPRESS `SELECT` types name *abstract* supertypes. `IfcBooleanOperand`
//! permits `IfcSolidModel`, but no file contains one -- files contain
//! `IfcExtrudedAreaSolid`, four levels below it. Answering "may this entity
//! stand in for that select member" therefore needs the inheritance chain.
//!
//! `ifc-schema` can parse the official `.exp` files and answer exactly this,
//! but requiring it would mean a geometry consumer must ship a 3 MB schema
//! file to interpret a wall. The chains for the geometry-reachable entities
//! are small and change only when the IFC schema does, so they are compiled in
//! as data.
//!
//! # Why the releases are separate rows, not one merged table
//!
//! IFC4X3 does not only add entities. It also inserts abstract supertypes
//! above four IFC4 entities: `IfcOffsetCurve2D` and `IfcOffsetCurve3D` gain
//! `IfcOffsetCurve`, `IfcFixedReferenceSweptAreaSolid` and
//! `IfcSurfaceCurveSweptAreaSolid` gain `IfcDirectrixCurveSweptAreaSolid`.
//! One merged chain for those names would be wrong for one release or the
//! other, so the IFC4X3 rows that differ live apart and are used only when a
//! caller names IFC4X3.
//!
//! The unversioned [`is_a`] and [`supertypes_of`] answer the IFC4 chain for
//! every entity IFC4 declares, exactly as before IFC4X3 rows existed, and the
//! IFC4X3 chain for entities only IFC4X3 declares. A name IFC4 lacks cannot
//! occur in an IFC4 file, so that fallback changes no IFC4 answer. The only
//! IFC4X3 answers it misses are the inserted abstract supertypes above: no
//! select or rule in this crate asks for them, and [`is_a_in`] answers them.
//!
//! # Keeping it honest
//!
//! Generated from `IFC4.exp` and `IFC4X3_ADD2.exp`. `tests/schema_coverage.rs`
//! compares every chain, per release, with the same normative sources, so
//! drift fails the build rather than silently misclassifying a solid.

use ifc_schema::SchemaVersion;

use super::subtype_ifc4::SUPERTYPES;
use super::subtype_ifc4x3;

/// The base release of the compiled tables.
///
/// The unversioned [`is_a`] and [`supertypes_of`] answer this release's chain
/// for every entity it declares. [`VERIFIED_SCHEMA_VERSIONS`] lists every
/// release [`is_a_in`] answers verbatim.
pub const TABLE_SCHEMA_VERSION: SchemaVersion = SchemaVersion::Ifc4;

/// Every release whose chains the compiled tables carry verbatim.
pub const VERIFIED_SCHEMA_VERSIONS: &[SchemaVersion] =
    &[SchemaVersion::Ifc4, SchemaVersion::Ifc4x3];

/// Does [`is_a_in`] answer subtype questions for `version` verbatim?
///
/// True for IFC4 and IFC4X3. False does not mean refusal: IFC4X1 and IFC4X2
/// share most geometry chains with these, so the unversioned answers
/// [`is_a_in`] falls back to are usually right. It means they are not
/// *verified* against that schema, and a caller that needs certainty must say
/// so.
pub fn tables_are_verified_for(version: SchemaVersion) -> bool {
    VERIFIED_SCHEMA_VERSIONS.contains(&version)
}

/// Is `entity` the named type, or any subtype of it?
///
/// The question every EXPRESS `SELECT` resolution reduces to. Comparing type
/// names directly instead of calling this rejects every real file, because
/// select members are usually abstract.
///
/// Release-neutral: answers the IFC4 chain where IFC4 declares `entity`, and
/// the IFC4X3 chain where only IFC4X3 does (see the module docs). Use
/// [`is_a_in`] to ask about one release exactly.
///
/// ```
/// use ifc_geometry::select::is_a;
/// // An extruded area solid IS a solid model, four levels up.
/// assert!(is_a("IFCEXTRUDEDAREASOLID", "IFCSOLIDMODEL"));
/// assert!(!is_a("IFCCARTESIANPOINT", "IFCSOLIDMODEL"));
/// // IFC4X3-only entities resolve too.
/// assert!(is_a("IFCCLOTHOID", "IFCCURVE"));
/// ```
pub fn is_a(entity: &str, ancestor: &str) -> bool {
    chain_contains(entity, supertypes_of(entity), ancestor)
}

/// Is `entity` the named type, or any subtype of it, in `version`?
///
/// Exact for every release [`tables_are_verified_for`] accepts; other
/// releases get the release-neutral [`is_a`] answer.
///
/// ```
/// use ifc_geometry::select::is_a_in;
/// use ifc_schema::SchemaVersion;
/// // IFC4X3 inserts IfcOffsetCurve above IfcOffsetCurve2D; IFC4 has none.
/// assert!(is_a_in(SchemaVersion::Ifc4x3, "IFCOFFSETCURVE2D", "IFCOFFSETCURVE"));
/// assert!(!is_a_in(SchemaVersion::Ifc4, "IFCOFFSETCURVE2D", "IFCOFFSETCURVE"));
/// ```
pub fn is_a_in(version: SchemaVersion, entity: &str, ancestor: &str) -> bool {
    chain_contains(entity, supertypes_of_in(version, entity), ancestor)
}

fn chain_contains(entity: &str, chain: &[&str], ancestor: &str) -> bool {
    entity.eq_ignore_ascii_case(ancestor) || chain.iter().any(|s| s.eq_ignore_ascii_case(ancestor))
}

/// The supertype chain of an entity, immediate parent first.
///
/// Release-neutral, as [`is_a`]. Empty for an unknown entity: a type from a
/// newer schema is not an error, it simply matches no select, which is the
/// correct conservative answer.
pub fn supertypes_of(entity: &str) -> &'static [&'static str] {
    find(ifc4_rows(), entity)
        .or_else(|| find(subtype_ifc4x3::ADDED.iter(), entity))
        .unwrap_or(&[])
}

/// The supertype chain of an entity in `version`, immediate parent first.
///
/// Exact for every release [`tables_are_verified_for`] accepts, and empty for
/// an entity that release does not declare. Other releases get the
/// release-neutral [`supertypes_of`] answer.
pub fn supertypes_of_in(version: SchemaVersion, entity: &str) -> &'static [&'static str] {
    match version {
        SchemaVersion::Ifc4 => find(ifc4_rows(), entity).unwrap_or(&[]),
        SchemaVersion::Ifc4x3 => find(subtype_ifc4x3::REDECLARED.iter(), entity)
            .or_else(|| find(ifc4_rows(), entity))
            .or_else(|| find(subtype_ifc4x3::ADDED.iter(), entity))
            .unwrap_or(&[]),
        _ => supertypes_of(entity),
    }
}

/// Every entity this table knows, for cross-checking against the schema.
///
/// The union over [`VERIFIED_SCHEMA_VERSIONS`], each name once.
pub fn known_entities() -> impl Iterator<Item = &'static str> {
    ifc4_rows()
        .chain(subtype_ifc4x3::ADDED.iter())
        .map(|(name, _)| *name)
}

/// Every entity `version` declares in these tables; empty for a release
/// [`tables_are_verified_for`] rejects.
pub fn known_entities_in(version: SchemaVersion) -> impl Iterator<Item = &'static str> {
    let added: &'static [Row] = match version {
        SchemaVersion::Ifc4x3 => subtype_ifc4x3::ADDED,
        _ => &[],
    };
    let base = tables_are_verified_for(version);
    ifc4_rows()
        .filter(move |_| base)
        .chain(added.iter())
        .map(|(name, _)| *name)
}

type Row = (&'static str, &'static [&'static str]);

fn ifc4_rows() -> impl Iterator<Item = &'static Row> {
    SUPERTYPES
        .iter()
        .chain(super::subtype_profile::PROFILE_SUPERTYPES.iter())
}

fn find<'a>(
    mut rows: impl Iterator<Item = &'a Row>,
    entity: &str,
) -> Option<&'static [&'static str]> {
    rows.find(|(name, _)| name.eq_ignore_ascii_case(entity))
        .map(|(_, chain)| *chain)
}

#[cfg(test)]
mod tests;
