# Changelog -- openbim-ifc

All notable changes to the `openbim-ifc` crate are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate follows Semantic Versioning independently of its siblings:
a release here does not imply a release of any other crate in the family.

Changes up to and including 0.2.0 are recorded family-wide in the
[repository changelog](../../CHANGELOG.md); that file is the archive for
everything released before per-crate changelogs began.

## [Unreleased]

### Changed (breaking)

- `validate`, `spatial` and `geometry-select` link only the releases the
  build names (#306): each release feature (`ifc2x3` ... `ifc4x3`, and
  `schema` for all five) now forwards to `ifc-validate`, `ifc-spatial` and
  `ifc-geometry`, which no longer pull every release's table in through
  `ifc-schema`'s defaults. An IFC4-only build with validation carries the
  IFC4 table alone. A build that enabled one of these features without a
  release feature must now add one (or `schema`): `spatial` fails to
  compile without IFC2X3, IFC4 or IFC4X3, `validate_declared` refuses
  every file with `UnbundledSchema`, and release-bound geometry authoring
  refuses with `AuthoringSchemaUnbound`. `full` and `domains` (which
  implies `schema` through `author`) are unchanged. Under 0.x, a minor
  release.

## [0.11.0] - 2026-10-03

### Changed (breaking)

- Behind `step`, the re-exported `ParseOptions` and `OnMalformed` are
  `openbim-step` 0.11's (were 0.8's), through `ifc-step` (#288). 0.11 adds
  `ParseOptions::accept_real_without_point`, included in
  `ParseOptions::lenient()`: a lenient read keeps a REAL written without
  its decimal point (`1E-05`) with a diagnostic instead of skipping its
  record. Strict reads still refuse it.

## [0.10.0] - 2026-10-02

### Changed (breaking)

- Behind `ifcxml`, the re-exported `XmlCodec` reads strictly by default when
  it has a schema: values are typed from the schema and undeclared names
  are refused (`ifc-xml` #266; see its changelog).
  `.with_reading(SchemaReading::Lenient)` restores the previous read.

### Added

- Behind `ifcxml`, re-exports `SchemaReading` and `XmlLayout` beside
  `XmlCodec` and `XmlProfile`, and with them the buildingSMART XSD layout
  reader `XmlCodec::xsd` (`ifc-xml` #265).

## [0.9.0] - 2026-09-29

### Added

- `tests/stationing_template.rs`: a referent's `Pset_Stationing` authored
  by `alignment` and read by `properties` checks clean against the
  `property-catalog` IFC4X3 ADD2 corrected profile (#216).

### Changed (breaking)

- `PanelPosition` and `Unreachable` are `#[non_exhaustive]`: a match needs a
  wildcard arm.
- `Sector` and `ContainerElements` are `#[non_exhaustive]`; they can no
  longer be built with a struct literal outside the crate.

- Code previously behind `schema` is behind `schema-api`, which `schema`
  and every release feature imply; `schema::for_version` returns a
  `Result` (see `ifc-schema`).

### Changed

- The georeferencing and alignment conformance test gives its placeholder
  `IfcPolyline` two points (`Points` is `LIST [2:?]`, checked since #111).
- Door and window operation reads name the type-object entity per
  verified release (IFC2X3, IFC4, IFC4X3) and refuse any other with
  `ExactPropertyError::UnsupportedSchema`, instead of treating every
  non-IFC2X3 release as IFC4.

### Added

- Per-release schema features (#112): `ifc2x3`, `ifc4`, `ifc4x1`, `ifc4x2`,
  `ifc4x3`, each providing the schema API with that one bundled table, and
  `schema-api` (the API with no table). `schema` keeps its meaning: the API
  with every release. A single-release build refuses the others through
  `schema::for_version` with `schema::NotBundled`. Domain features link the
  releases their crates read, so enabling one brings every release it
  reads.
- `compiled_features()` reports `schema-api` and each release feature.
- `tests/intermediate_releases.rs`: IFC4X1 and IFC4X2 files resolve to
  their own `SchemaVersion`, release id and bundled table (#33).

## [0.8.1] - 2026-09-28

### Changed

- Requires the patch releases published with it: `ifc-properties` 0.5.1
  (bare quantity values, IFC2X3 `*_with_owner_history` authoring, type
  objects in the exact API), `ifc-cost` 0.2.2, `ifc-classification` 0.2.2
  (IFC4X3 binding) and `ifc-systems` 0.2.2.
- `door_operation` and `window_operation` given a type object (an
  `IfcDoorType`, IFC2X3 `IfcDoorStyle`, ...) refuse it with `NotADoor` or
  `NotAWindow`, where they returned `Property(InvalidQueryObject)`: the
  exact property reader now accepts type objects (ifc-properties #193).
  Results and refusals for doors and windows are unchanged.

## [0.8.0] - 2026-09-28

### Changed (breaking)

- `properties` re-exports `ifc-properties` 0.5.0, whose breaking changes
  pass through (see that crate's changelog):
  - `Quantity` and `QuantityKind` are `#[non_exhaustive]`, and gain
    `Quantity::Unresolved` for a quantity without a readable value (#138)
    and `QuantityKind::Number` for IFC4X3 `IfcQuantityNumber`;
  - `create_quantity` / `create_quantity_with` take the `&Model` they write
    into and write its release's layout;
  - `PropertyTemplate` gains fields and is `#[non_exhaustive]` (#108).
- Requires `ifc-spatial` 0.2.3 (`referenced_elements`, release-bound
  container classification) and `ifc-geometry` 0.4.3
  (`BodyItem::item_world`, #185).

### Added

- `spatial_properties(model)` (features `spatial` and `properties`): every
  spatial container in tree order, depth first from the roots, with the
  elements it holds, each with its `exact_properties` list, in IFC2X3, IFC4
  and IFC4X3 (#121). An element is listed as `Contained`
  (`IfcRelContainedInSpatialStructure`), `Referenced`
  (`IfcRelReferencedInSpatialStructure`, so an element spanning several
  storeys appears under each) or `Part` (an `IfcRelAggregates` part, at any
  depth, of a contained element, which the Element Composition concept
  places by its composite's containment), ordered by element id. A nested
  space is its own container, not folded into its storey. A model-level
  refusal (diagnostics, missing or unsupported schema) is the function's
  error; any other `ExactPropertyError` is reported on the element it
  concerns and the other elements are still returned. Properties resolve
  lazily as `ContainerElements::elements` (or `elements_where`, with
  `exact_properties_where` selectors) is iterated. New types:
  `SpatialProperties`, `ContainerElements`, `SpatialContainer`,
  `ContainerName`, `ElementMember`, `ElementProperties` and
  `SpatialMembership`. Needs the next `ifc-spatial` release, which adds
  `SpatialTree::referenced_elements`.

### Changed

- The `spatial` feature classifies spatial containers from the file's
  declared release (#121, via the next `ifc-spatial` release, which now
  links `ifc-schema`): IFC4X3 facilities and facility parts such as
  `IfcRoad`, `IfcRoadPart`, `IfcBridge` and `IfcBridgePart`, and
  `IfcExternalSpatialElement`, are containers, so `SpatialTree`,
  `spatial_properties` and `unreachable_products` see the elements placed
  in them. Containment or reference into a non-container is reported as a
  `SpatialAnomaly`. `unreachable_products` skips containers by the tree's
  classification instead of a name test.

## [0.7.3] - 2026-09-27

### Changed

- The crate README, which is the crates.io page, is rewritten. The published
  one told readers to depend on a Git revision and said the crates were not
  on crates.io. The code is unchanged since 0.7.2.

## [0.7.2] - 2026-09-27

### Added

- `window_operation(model, window)` (features `geometry-select` and
  `properties`), the window counterpart of `door_operation`: each panel of
  a window as a world frame, width, height, hinge side, swing `Sector` for
  a side hinge and tilt `Sector` for a top or bottom hinge, from its
  placement, `OverallWidth`/`OverallHeight`, partitioning
  (`IfcWindowType.PartitioningType`, IFC2X3 `IfcWindowStyle.OperationType`,
  or the occurrence's), `IfcWindowPanelProperties` and the mullion and
  transom offsets of `IfcWindowLiningProperties`, in IFC2X3, IFC4 and
  IFC4X3 (#170). Single, double and triple partitionings with side-hung,
  tilt-and-turn, top- and bottom-hung, sliding, removable and fixed panels
  are derived; `NOTDEFINED` and `USERDEFINED` partitionings, pivot,
  `OTHEROPERATION` and `NOTDEFINED` panels, a window without panel
  properties or overall size, panels that contradict the partitioning, and
  a split without a valid lining offset are refused as
  `WindowOperationError`, never defaulted. Panels tile the placement's XZ
  plane; lining, mullion and transom thicknesses are not applied.
  `Sector` and `Side` are shared with `door_operation`, unchanged.

## [0.7.1] - 2026-09-27

### Added

- `profile_outline` and `ProfileOutline` are re-exported at the root under
  `geometry-select` (#166), beside `describe_profile`: the straight-edged
  outline of an arbitrary closed profile, without the geometry kernel.

### Changed

- Requires `ifc-geometry` 0.4.1, which carries `profile_outline`,
  `product_representation_frame` and the curve-bounded plane fix (#163).

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
[repository changelog](../../CHANGELOG.md) for the family-wide history
that produced this version.

[Unreleased]: https://github.com/openbimrs/ifc/compare/openbim-ifc-v0.11.0...HEAD
[0.11.0]: https://github.com/openbimrs/ifc/releases/tag/openbim-ifc-v0.11.0
[0.10.0]: https://github.com/openbimrs/ifc/releases/tag/openbim-ifc-v0.10.0
[0.9.0]: https://github.com/openbimrs/ifc/releases/tag/openbim-ifc-v0.9.0
[0.8.1]: https://github.com/openbimrs/ifc/releases/tag/openbim-ifc-v0.8.1
[0.8.0]: https://github.com/openbimrs/ifc/releases/tag/openbim-ifc-v0.8.0
[0.7.3]: https://github.com/openbimrs/ifc/releases/tag/openbim-ifc-v0.7.3
[0.7.2]: https://github.com/openbimrs/ifc/releases/tag/openbim-ifc-v0.7.2
[0.7.1]: https://github.com/openbimrs/ifc/releases/tag/openbim-ifc-v0.7.1
[0.7.0]: https://github.com/openbimrs/ifc/releases/tag/openbim-ifc-v0.7.0
[0.6.0]: https://github.com/openbimrs/ifc/releases/tag/openbim-ifc-v0.6.0
[0.5.0]: https://github.com/openbimrs/ifc/releases/tag/openbim-ifc-v0.5.0
[0.4.0]: https://github.com/openbimrs/ifc/releases/tag/openbim-ifc-v0.4.0
[0.3.1]: https://github.com/openbimrs/ifc/releases/tag/openbim-ifc-v0.3.1
[0.3.0]: https://github.com/openbimrs/ifc/releases/tag/openbim-ifc-v0.3.0
[0.2.0]: https://github.com/openbimrs/ifc/releases/tag/v0.2.0
