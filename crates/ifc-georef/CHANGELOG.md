# Changelog -- ifc-georef

All notable changes to the `ifc-georef` crate are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate follows Semantic Versioning independently of its siblings:
a release here does not imply a release of any other crate in the family.

Changes up to and including 0.2.0 are recorded family-wide in the
[repository changelog](../../CHANGELOG.md); that file is the archive for
everything released before per-crate changelogs began.

## [Unreleased]

### Added

- `site_reference` reads `IfcSite.RefLatitude`, `RefLongitude` and
  `RefElevation` (metres, through the project length scale) under IFC2X3,
  IFC4 and IFC4X3 into `SiteReference`; compound angles are validated
  against the declared release's own WHERE rules and the WGS84 range, and
  convert to decimal degrees via `CompoundPlaneAngle` (#242).
- `relate_site_elevation` compares `RefElevation` with the map height of
  the site origin (`OrthogonalHeight` plus the scaled site-origin height)
  and returns `SiteElevationCheck`: a disagreement beyond the tolerance
  (`SITE_ELEVATION_TOLERANCE_M`, 1 cm suggested) is a reported finding
  carrying both values and the target CRS's `VerticalDatum`, never a
  silent pick (#242).
- `GeorefError::InvalidCompoundAngle` and `GeorefError::InvalidParameter`
  (#242).

## [0.4.0] - 2026-09-29

### Added

- `ProjectedCrs.well_known_text`: the OGC WKT literal of the one IFC4X3
  `IfcWellKnownText` defining the CRS, verbatim (#142).
- `GeorefError::RuleViolation { entity, rule }` for a schema WHERE rule or
  inverse cardinality a record breaks.

### Changed (breaking)

- `ProjectedCrs.name` is `Option<String>` (#142). IFC4X3 declares
  `IfcCoordinateReferenceSystem.Name : OPTIONAL IfcLabel` with
  `WHERE NameOrWKT : (HIINDEX(WellKnownText) = 1) OR EXISTS(Name)`: under an
  IFC4X3 header an unnamed CRS defined by exactly one `IfcWellKnownText` now
  reads, one with neither is refused with the new
  `GeorefError::RuleViolation { rule: "NameOrWKT" }`, and two definitions
  for one CRS (the inverse is `SET [0:1]`) are refused too. IFC4, or a
  missing or ambiguous header, keeps requiring the name
  (`MissingAttribute`), so `name` is always `Some` there. Migrate with
  `crs.name.as_deref()`.
- The authoring drafts `ProjectedCrsDraft`, `GeographicCrsDraft` and
  `MapConversionDraft` are `#[non_exhaustive]`: build them with
  `ProjectedCrsDraft::new(name)`, `GeographicCrsDraft::new()` or
  `MapConversionDraft::new(source_crs, target_crs, eastings, northings,
  orthogonal_height)` and a setter named after each optional field
  (`.map_unit(unit)`, `.x_axis((abscissa, ordinate))`, `.scale(s)`). Fields
  stay public.
- The read-side `ProjectToMap` and `ProjectedCrs` are `#[non_exhaustive]`;
  they can no longer be built with a struct literal outside the crate.

### Changed

- Depends on `ifc-schema` with its default features named explicitly
  (every bundled release), now that the workspace dependency turns them
  off for the facade's per-release features (#112).
- A model whose header declares `IFC4X1` or `IFC4X2` is refused with the
  existing unsupported-schema error. `ifc-schema` now bundles both
  releases, but no layout here is verified against them, so they are
  never read as IFC4 or IFC4X3.

## [0.3.0] - 2026-09-23

### Changed

- **Breaking:** requires Axiolid 0.3. `ProjectToMap::transform` is an
  `axiolid_core::Transform3`, so the major Axiolid version is part of this
  crate's public API. No code change.

## [0.2.0] - 2026-09-22

First release under per-crate versioning. See the
[repository changelog](../../CHANGELOG.md) for the family-wide history
that produced this version.

[Unreleased]: https://github.com/openbimrs/ifc/compare/ifc-georef-v0.4.0...HEAD
[0.4.0]: https://github.com/openbimrs/ifc/releases/tag/ifc-georef-v0.4.0
[0.3.0]: https://github.com/openbimrs/ifc/releases/tag/ifc-georef-v0.3.0
[0.2.0]: https://github.com/openbimrs/ifc/releases/tag/v0.2.0
