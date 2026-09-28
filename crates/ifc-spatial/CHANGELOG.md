# Changelog -- ifc-spatial

All notable changes to the `ifc-spatial` crate are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate follows Semantic Versioning independently of its siblings:
a release here does not imply a release of any other crate in the family.

Changes up to and including 0.2.0 are recorded family-wide in the
[repository changelog](../../CHANGELOG.md); that file is the archive for
everything released before per-crate changelogs began.

## [Unreleased]

### Added

- A `*_with_owner_history` variant of every `IfcRoot` writer (#202):
  `create_spatial_element`, `create_project`, `aggregate`, `contain`,
  `create_external_spatial_element`, `create_project_library`,
  `create_space_boundary`, `connect_path_elements`, `create_facility`, and
  in `authoring` `cover_elements`, `cover_spaces`, `declare`,
  `define_by_object`, `serve_buildings`, `control_flow_element`,
  `assign_to_actor`, `assign_to_product`, `assign_to_process`,
  `assign_to_group_by_factor`, `assign_to_resource`, `connect_elements`,
  `connect_with_realizing_elements`, `interfere_elements`, `void_element`,
  `fill_element`, `project_element`, `adhere_to_element`,
  `position_products` and `associate_profile_def`. Each takes the model
  and a caller-supplied `IfcOwnerHistory` id, which IFC2X3 requires on
  every `IfcRoot`; the id must be in the model or staged on the
  transaction and be an `IfcOwnerHistory`, and none is ever invented. Each
  binds the model's declared release as `ifc-material` (#77),
  `ifc-properties` (#191) and `ifc-classification` (#194) do (no
  `FILE_SCHEMA` binds IFC4) and lays the record out by attribute name
  from that release's table: IFC2X3 `IfcRelCoversSpaces.RelatingSpace` is
  written as `RelatedSpace`, IFC4's name for the same attribute. What the
  release cannot hold is refused: entities it does not declare
  (`IfcRelDeclares`, `IfcRelDefinesByObject`,
  `IfcRelAssignsToGroupByFactor`, `IfcRelInterferesElements`,
  `IfcExternalSpatialElement`, `IfcProjectLibrary` and the 1st- and
  2nd-level space boundaries in IFC2X3; `IfcRelAdheresToElement`,
  `IfcRelPositions`, `IfcRelAssociatesProfileDef` and the facilities
  outside IFC4X3), enumeration tokens outside its table, and attributes it
  requires but the call cannot supply (IFC2X3 `CompositionType`,
  `IfcSpace.InteriorOrExteriorSpace`, `IfcProject.RepresentationContexts`).
  In IFC4 and IFC4X3 each variant writes the plain writer's record with
  the reference in slot 1.
- `SpatialAuthoringError::MultipleSchemas`, `UnsupportedSchema`,
  `EntityNotInSchema`, `AuthoringNotInSchema`, `AuthoringValueType`,
  `AuthoringRequired`, `MissingReference` and `WrongReferenceType`, and
  `FacilityError::Authoring` wrapping them (#202). Both enums are
  `#[non_exhaustive]`, so this is not breaking.

### Changed

- `create_space_boundary` binds the model's declared release and lays its
  record out by attribute name (#202). It wrote the IFC4 layout with
  `OwnerHistory` `$` into every model, and IFC2X3 requires the owner
  history. **Behaviour change for IFC2X3 callers:** it now refuses an
  IFC2X3 model with `SpatialAuthoringError::AuthoringRequired { attribute:
  "OwnerHistory", .. }` and stages nothing; use
  `create_space_boundary_with_owner_history`. It also refuses a header
  declaring several schemas (`MultipleSchemas`) or one without a bundled
  table (`UnsupportedSchema`), an IFC2X3 1st- or 2nd-level boundary
  (`EntityNotInSchema`) and a token the release does not declare, such as
  IFC2X3 `EXTERNAL_EARTH` (`AuthoringValueType`). IFC4 and IFC4X3 records
  are unchanged, slot for slot.

### Documented

- The writers that take no model cannot see the release: they write the
  IFC4/IFC4X3 layout with `OwnerHistory` `$` and are documented as
  IFC4/IFC4X3 only, pointing to their `*_with_owner_history` variant.
  Their output is unchanged. Four of them disagree with the release's
  arity, which the tests pin and the variants do not repeat:
  `assign_to_actor` and `assign_to_process` write seven of eight
  attributes, `connect_with_realizing_elements` eight of nine, and
  `interfere_elements` ten where IFC4 declares nine.

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
