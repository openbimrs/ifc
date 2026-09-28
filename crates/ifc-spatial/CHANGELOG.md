# Changelog -- ifc-spatial

All notable changes to the `ifc-spatial` crate are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate follows Semantic Versioning independently of its siblings:
a release here does not imply a release of any other crate in the family.

Changes up to and including 0.2.0 are recorded family-wide in the
[repository changelog](../../CHANGELOG.md); that file is the archive for
everything released before per-crate changelogs began.

## [Unreleased]

## [0.2.3] - 2026-09-28

### Added

- `SpatialTree::referenced_elements(container)` and
  `SpatialTree::referencing_structures(element)` (#121): the elements an
  `IfcRelReferencedInSpatialStructure` references in a container, and the
  containers referencing an element, in file order and each once, in
  IFC2X3, IFC4 and IFC4X3. They are kept apart from containment:
  `elements_of` and `container_of` are unchanged, and a referenced element
  is never a second home or a `ContainedTwice` anomaly. A reference naming
  an absent entity is reported by `dangling()`.
- `SpatialAnomaly::ContainedInNonContainer` and
  `SpatialAnomaly::ReferencedInNonContainer` (#121): an
  `IfcRelContainedInSpatialStructure` or `IfcRelReferencedInSpatialStructure`
  whose `RelatingStructure` is not a spatial container of the release is
  reported with the relationship and the structure, where containment used
  to drop it silently.
- `SpatialKind::classify_in(type_name, release)`, `SpatialTree::release()`
  and a re-export of `ifc_schema::SchemaVersion`.

### Changed

- Spatial containers are classified from the release the file's
  `FILE_SCHEMA` declares (#121): an entity is a container when that
  release's bundled table makes it an `IfcSpatialElement` (IFC2X3:
  `IfcSpatialStructureElement`), or it is the `IfcProject`. `ifc-schema` is
  therefore a normal dependency. The IFC4X3 facilities and facility parts
  (`IfcFacility`, `IfcBridge`, `IfcRoad`, `IfcRailway`,
  `IfcMarineFacility`, `IfcBridgePart`, `IfcRoadPart`, `IfcRailwayPart`,
  `IfcMarinePart`, `IfcFacilityPartCommon`) and `IfcExternalSpatialElement`
  were classified as elements by the old name patterns, so containment into
  them was dropped; they are now `OtherContainer`, as that variant's
  documentation promised. `SpatialKind` gains no variant, so this stays
  additive. A file with no single bundled release is classified as any
  bundled release would, and `release()` returns `None`.
- `SpatialKind::classify` answers from the bundled tables instead of name
  patterns: a name no release declares as a spatial element (such as a
  vendor `IFCSPATIALFOO`) is an `Element`.

### Fixed

- Only `IfcRelAggregates` and `IfcRelContainedInSpatialStructure` build the
  tree (#121). Another relationship family whose relating end is a
  container placed its targets as contained elements: an `IfcRelDeclares`
  put the project's declared types into the project, and an
  `IfcRelCoversSpaces` put a space's coverings into the space. Their absent
  targets are still reported by `dangling()`.

## [0.2.2] - 2026-09-27

### Added

- `SpaceBoundary::connection_geometry(&Model)` and
  `ConnectionGeometryAnomaly` (#156). The accessor returns the
  `ConnectionGeometry` reference (slot 6) of `IfcRelSpaceBoundary`,
  `IfcRelSpaceBoundary1stLevel` and `IfcRelSpaceBoundary2ndLevel`, in
  IFC2x3, IFC4 and IFC4X3. Its coordinates are in the relating space's
  object placement. `$` or a missing slot is `Ok(None)`. A dangling
  reference, a reference to something that is not a concrete
  `IfcConnectionGeometry` subtype, a value that is not a reference, and a
  boundary absent from the model are each an `Err` naming the boundary
  and, where there is one, the target. The accessor is a method rather
  than a new field so that `SpaceBoundary`, which has only public fields,
  keeps its struct-literal construction and this change stays additive.

## [0.2.1] - 2026-09-26

### Added

- `SpatialTree::anomalies()` and `SpatialAnomaly` (#54). An element placed
  by two `IfcRelContainedInSpatialStructure`s is reported as
  `ContainedTwice`, and a container aggregated by two parents as
  `AggregatedTwice`. Each names the element or child, the kept and rejected
  parent, and the rejected relationship. The first relationship applied
  still wins. Restating the same parent is not reported.

### Changed

- For invalid files only: the rejected container no longer lists a doubly
  contained element in `SpatialNode::elements` / `elements_of`. Before, the
  element appeared in both containers while `container_of` returned only the
  first, so the two views disagreed. Valid files are unaffected.

## [0.2.0] - 2026-09-22

First release under per-crate versioning. See the
[repository changelog](../../CHANGELOG.md) for the family-wide history
that produced this version.

[Unreleased]: https://github.com/openbimrs/ifc/compare/ifc-spatial-v0.2.3...HEAD
[0.2.3]: https://github.com/openbimrs/ifc/releases/tag/ifc-spatial-v0.2.3
[0.2.2]: https://github.com/openbimrs/ifc/releases/tag/ifc-spatial-v0.2.2
[0.2.1]: https://github.com/openbimrs/ifc/releases/tag/ifc-spatial-v0.2.1
[0.2.0]: https://github.com/openbimrs/ifc/releases/tag/v0.2.0
