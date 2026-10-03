# Changelog -- ifc-georef

All notable changes to the `ifc-georef` crate are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate follows Semantic Versioning independently of its siblings:
a release here does not imply a release of any other crate in the family.

Changes up to and including 0.2.0 are recorded family-wide in the
[repository changelog](../../CHANGELOG.md); that file is the archive for
everything released before per-crate changelogs began.

## [Unreleased]

## [0.5.1] - 2026-10-03

### Added

- `ProjectToMap::declared_map_unit()`: the target `IfcProjectedCRS.MapUnit`
  exactly as authored, `None` when the file leaves it unset, next to the
  resolved `ProjectToMap::map_unit`, which keeps the project length unit
  as the default for an omitted `MapUnit` and is what the transform uses.
  A CRS stating the project's unit is now distinguishable from one stating
  none, under IFC4 and IFC4X3, with or without the `transform` feature
  (#296).

### Changed

- Documented that `ProjectedCrs::map_unit` is the declared `MapUnit`
  (`None` when unset), never filled in with the project default (#296).

## [0.5.0] - 2026-10-02

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
- IFC4X3 `IfcRigidOperation` with `IfcLengthMeasure` coordinates resolves
  to a translation through `resolve_project_to_map(_in)`: the offsets and
  `Height` are in the target `MapUnit`, and the linear part is the identity
  in metres (#241). `WHERE SameCoordinateType` is checked; an untyped REAL,
  mixed or non-length/non-angle coordinates are refused with
  `GeorefError::RuleViolation { rule: "SameCoordinateType" }`, and a
  length operation onto a non-projected target with `WrongType`.
- `resolve_geographic_offset_in` and `GeographicOffset`: an IFC4X3
  plane-angle `IfcRigidOperation` read as authored onto its
  `IfcGeographicCRS` (#241). The project-to-map resolver refuses that form
  with the new `GeorefError::CoordinateMeasureMismatch`, since no metre
  transform expresses a latitude/longitude offset.
- `GeographicCrs` and `AngleUnit`: IFC4X3 `IfcGeographicCRS` with
  `PrimeMeridian`, `AngleUnit` (`PLANEANGLEUNIT`) and `HeightUnit`
  (`LENGTHUNIT`); an absent unit stays `None` (#241).
- `create_angular_rigid_operation`, writing `IFCPLANEANGLEMEASURE(..)`
  coordinates (#241).
- `OperationSource`, `resolve_operation_source` and
  `coordinate_operation_for`: the operation's `SourceCRS` validated as an
  `IfcCoordinateReferenceSystemSelect` member of the pinned release, and
  the `HasCoordinateOperation` inverse read from a context or CRS (#101).
- `ProjectToMap.source`, `.operation` and `.kind` (`OperationKind`).
- A default `transform` feature carrying the `axiolid-core` dependency
  (#268). With `default-features = false` the crate links no geometry
  crate and still resolves CRS metadata and units, every map-conversion
  and rigid-operation parameter, true and grid north, operation sources
  and the site reference. `tests/kernel_free_build.rs` asserts the
  resolved dependency graph, and the gate builds, tests, lints and
  documents the column.
- `ProjectToMap.eastings`, `.northings` and `.orthogonal_height`, as
  authored in `map_unit` (a rigid operation's `FirstCoordinate`,
  `SecondCoordinate` and `Height`, `0.0` when unstated), and the
  plain-number operation: `ProjectToMap::map_point` (project metres to map
  metres, bit-identical to `transform.transform_point3`),
  `ProjectToMap::linear_part` and `ProjectToMap::translation` (#268).

### Changed

- `ProjectToMap.transform` and `compose_project_frame` exist only with the
  `transform` feature, which is on by default, so a default build is
  source-compatible (#268). `relate_site_elevation` computes through
  `ProjectToMap::map_point` and is available in both columns, with the
  same results.
- `IfcMapConversionScaled` resolves instead of being refused with
  `UnsupportedOperation`: `FactorX/Y/Z` scale the source axes before the
  rotation, and a non-positive factor is refused with `InvalidAttribute`
  (#241). `grid_north_direction` accounts for unequal `FactorX`/`FactorY`.
- `create_rigid_operation` writes its coordinates as
  `IFCLENGTHMEASURE(..)` instead of bare REALs, which cannot satisfy
  `SameCoordinateType` (#241).
- Every resolved operation now validates its `SourceCRS` (#101): a
  dangling reference is `MissingEntity`, a wrong type `WrongType`, a
  sub-context `RuleViolation { rule: "NoCoordOperation" }`, and a source
  named by more than one operation `RuleViolation` on the
  `HasCoordinateOperation : SET [0:1]` inverse. Files that resolved with
  such a source before are now refused.
- `resolve_project_to_map` pins the header's release when it names IFC4 or
  IFC4X3, so IFC4X3-only entities in an IFC4 file are refused as
  undeclared by both entry points. A non-coordinate-operation id is
  `WrongType { expected: "IFCCOORDINATEOPERATION" }` (was
  `"IFCMAPCONVERSION"`).
- `create_map_conversion` and `create_map_conversion_scaled` refuse a
  negative `Scale`, and `create_map_conversion_scaled` a negative
  `FactorX`/`FactorY`/`FactorZ`, with `AuthoringInvalid` naming the
  attribute and nothing staged (#254). Zero and non-finite values were
  already refused. The reader refuses all of these (`InvalidScale`,
  `InvalidAttribute`), so a caller that passed one got a record this
  crate could not read back; such calls now fail at authoring time.

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

[Unreleased]: https://github.com/openbimrs/ifc/compare/ifc-georef-v0.5.1...HEAD
[0.5.1]: https://github.com/openbimrs/ifc/releases/tag/ifc-georef-v0.5.1
[0.5.0]: https://github.com/openbimrs/ifc/releases/tag/ifc-georef-v0.5.0
[0.4.0]: https://github.com/openbimrs/ifc/releases/tag/ifc-georef-v0.4.0
[0.3.0]: https://github.com/openbimrs/ifc/releases/tag/ifc-georef-v0.3.0
[0.2.0]: https://github.com/openbimrs/ifc/releases/tag/v0.2.0
