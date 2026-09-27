# Changelog -- openbim-ifc

All notable changes to the `openbim-ifc` crate are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate follows Semantic Versioning independently of its siblings:
a release here does not imply a release of any other crate in the family.

Changes up to and including 0.2.0 are recorded family-wide in the
[repository changelog](../CHANGELOG.md); that file is the archive for
everything released before per-crate changelogs began.

## [Unreleased]

### Added

- `profile_outline` and `ProfileOutline` are re-exported at the root under
  `geometry-select` (#166), beside `describe_profile`: the straight-edged
  outline of an arbitrary closed profile, without the geometry kernel.

## [0.7.0] - 2026-09-27

### Changed (breaking)

- Re-exports three crates whose breaking releases pass through:
  `validate` is `ifc-validate` 0.3.0 (`Severity::EvaluationError`, renamed
  rule ids, `Budget::max_depth` removed), `material` is `ifc-material` 0.3.0
  (views and authoring bound to the declared release; `create_material`
  takes the model), and `geometry` is `ifc-geometry` 0.4.0 (typed reference
  errors, `IfcBlock` meshes no longer offset by half their extents). See
  each crate's changelog.
- Requires the releases published with it: `ifc-properties` 0.4.1,
  `ifc-spatial` 0.2.2, `ifc-xml` 0.2.1, `ifc-alignment` 0.3.1,
  `ifc-author` 0.2.1, `ifc-control` 0.2.1, `ifc-cost` 0.2.1,
  `ifc-tabular` 0.2.1 and `ifc-template-catalog` 0.2.1.

### Added

- `body_description`, `describe_profile` and their types (`BodyDescription`,
  `BodyItem`, `BodyKind`, `SweptSolid`, `SweepPath`, `ProfileDescription`,
  `ProfileParameters`) are re-exported at the root under `geometry-select`
  (#147), so a rule check reads a body's kind and swept-solid profile
  parameters without linking the geometry kernel.
- `door_operation(model, door)` (features `geometry-select` and
  `properties`): each leaf of a door as a world frame, width, hinge side and
  swing `Sector`, from its placement, `OperationType` and
  `IfcDoorPanelProperties`, in IFC2X3, IFC4 and IFC4X3 (#148). Single and
  double swing, double-acting, sliding, rolling-up and swing-fixed doors are
  derived; `NOTDEFINED`, `USERDEFINED`, revolving, folding, lifting and the
  `DOUBLE_DOOR_SINGLE_SWING_OPPOSITE_*` operations, a door without panel
  properties or `OverallWidth`, and panels that contradict the operation are
  refused as `DoorOperationError`, never defaulted. The leaves lie on the
  placement's x axis; lining offsets across the wall depth are not applied.

### Changed

- The `properties` feature also names `ifc-schema`, which `ifc-properties`
  already links, so the door join reads attributes by name from the bound
  release's table. No crate is added to a build.

## [0.6.0] - 2026-09-26

### Changed (breaking)

- `properties` re-exports `ifc-properties` 0.4.0, whose breaking changes
  (`PropertyAnomaly` is `#[non_exhaustive]`; `template_of_set` returns every
  template of a set) pass through. It also resolves quantity sets in
  `exact_property` instead of reporting them absent (#66).

### Changed

- STEP models load lazily: `from_step_bytes`, `read_path` and every strict
  read validate the whole file but decode each entity on first access
  (ADR 0015, see `ifc-step`). `read_path` hands its buffer to the codec
  instead of letting it copy the file once more.

### Added

- `StepReader`, `ParseOptions` and `OnMalformed` are re-exported, so the
  eager and memory-mapped reads (`StepReader::eager`,
  `StepReader::read_path_mapped`) are reachable through the facade.

### Added

- `Transaction`, `Applied` and `Conflict` are re-exported. `EntityEditor` and
  the domain writers stage into a `Transaction`, which facade users could not
  name, so an editor could be built but never applied.

## [0.5.0] - 2026-09-26

### Added

- `ifc::properties::exact_unit` (ifc-properties 0.3): a measure's effective
  unit resolved exactly to SI, or refused (#53).
- `ifc::spatial::SpatialTree::anomalies` (ifc-spatial 0.2.1): double
  containment and double aggregation are reported (#54).

### Changed

- **Breaking:** re-exports ifc-properties 0.3, whose
  `UnitKind::Si::prefix_exponent` is now `Option<i32>` and whose
  `UnitKind::Conversion` gains an `offset` field.

## [0.4.0] - 2026-09-23

### Added

- Textured tessellated geometry: `ifc::geometry` (ifc-geometry 0.3) lowers
  `IfcIndexedTriangleTextureMap` as a per-corner `uv` channel that survives
  compilation, including multi-item bodies and mirrored `IfcMappedItem`s
  (#30, axiolid/kernel#115).

### Changed

- **Breaking:** re-exports ifc-geometry 0.3, ifc-georef 0.3 and
  ifc-alignment 0.3, which all require Axiolid 0.3.

## [0.3.1] - 2026-09-23

### Added

- `ifc::schema` re-exports `ifc-schema` (feature `schema`), so the bundled
  schemas `ids_of_type_including_subtypes` needs, such as
  `ifc::schema::ifc4()`, are reachable without a direct `ifc-schema`
  dependency. Found by building a crates.io-only consumer of 0.3.0.

## [0.3.0] - 2026-09-23

### Added

- `ids_of_type_including_subtypes` (feature `schema`): every entity of a type
  or any of its subtypes, in file order. `Model::ids_of_type("IfcElement")`
  returns nothing because no instance is declared as the abstract supertype;
  this answers the question that call looks like it should. The caller passes
  the `Schema`, because the tree differs by version (#32).

### Changed

- **Breaking:** requires `ifc-style` 0.3.0, re-exported as `ifc::style`.
  `IndexedTextureMap::maps` there now returns `Vec<EntityId>`.

## [0.2.0] - 2026-09-22

First release under per-crate versioning. See the
[repository changelog](../CHANGELOG.md) for the family-wide history
that produced this version.

[Unreleased]: https://github.com/openbimrs/ifc/compare/openbim-ifc-v0.7.0...HEAD
[0.7.0]: https://github.com/openbimrs/ifc/releases/tag/openbim-ifc-v0.7.0
[0.6.0]: https://github.com/openbimrs/ifc/releases/tag/openbim-ifc-v0.6.0
[0.5.0]: https://github.com/openbimrs/ifc/releases/tag/openbim-ifc-v0.5.0
[0.4.0]: https://github.com/openbimrs/ifc/releases/tag/openbim-ifc-v0.4.0
[0.3.1]: https://github.com/openbimrs/ifc/releases/tag/openbim-ifc-v0.3.1
[0.3.0]: https://github.com/openbimrs/ifc/releases/tag/openbim-ifc-v0.3.0
[0.2.0]: https://github.com/openbimrs/ifc/releases/tag/v0.2.0
