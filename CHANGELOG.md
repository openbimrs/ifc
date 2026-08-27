# Changelog

All notable changes to the OpenBIM.rs IFC family are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project follows Semantic Versioning.

## [Unreleased]

### Added

- Opt-in recovery from damaged exports. `StepCodec::lenient()` returns a
  `StepReader` that skips unreadable data records instead of failing the file,
  and `Model::diagnostics()` reports each dropped range, so a viewer can show
  "loaded, 1 record skipped" rather than silently losing data or refusing a
  2.5 MB model over one truncated record. `StepCodec` stays strict: an
  authoring tool that drops entities corrupts the file it edits. Header
  structure and the physical-file marker remain fatal under both policies.
- `ifc_model::Diagnostic`: codec-neutral non-fatal findings carried on the
  model, with an optional source byte range.
- Advanced `openbim-step` to `0.3.2` for the recovery API.

### Changed

- `ifc-geometry` representation contexts: `RepresentationContext` reads
  `IfcGeometricRepresentationContext` and `IfcGeometricRepresentationSubContext`
  -- identifier, type, parent, target scale, and a typed `TargetView` that
  preserves unknown enumeration constants instead of flattening them.
  `plan_contexts` finds the sub-contexts a drawing is authored into.
- DERIVED attribute inheritance: a sub-context redeclares six inherited
  attributes, which real files write as `*` meaning "read this from my parent".
  `precision`, `world_coordinate_system`, `coordinate_space_dimension` and
  `true_north` resolve the parent chain; reading the slot directly yields the
  marker and silently loses the project's precision and placement. See ADR 0009.
- `select_plan_representation`: the inverse of `select_shape_representation`.
  Prefers an explicit `PLAN_VIEW` context, then `Plan`/`Annotation`/`FootPrint`/
  `Axis`, and returns `None` for a solid-only product rather than offering a
  body to draw flat.

- `ifc-spatial`: containment and objectified relationship traversal. Builds the
  project/site/building/storey/element tree from `IfcRelAggregates`,
  `IfcRelContainedInSpatialStructure` and `IfcRelNests`, answering "which
  elements are on this storey" and its inverse. Tolerates real exports --
  omitted levels, elements on the building, duplicate storeys, dangling
  references and containment cycles -- reporting defects through `orphans()`
  and `dangling()`. Reached through the facade's new `spatial` feature.
  See ADR 0008.
- `ifc-model::ReverseIndex`: on-demand target-to-referrer index recording the
  attribute slot each reference sits in, which is what distinguishes the two
  ends of an objectified relationship.
- `ifc-model` bounded traversal: `depth_first`, `breadth_first` and
  `find_cycle` with explicit `Budget`/`Stop` reporting, so a malformed file
  truncates with a diagnosis instead of hanging the caller.

- `ifc-author`: schema-checked entity construction. Build an entity by naming
  its attributes and let `ifc-schema` resolve STEP slot positions, instead of
  hand-placing positional values. Refuses unknown entities and attributes,
  duplicate sets, missing required attributes, declared-type and aggregate
  mismatches, and malformed GlobalIds. Reached through the facade's new
  `author` feature. See ADR 0007.

- A documentation site (VitePress) published to GitHub Pages, covering
  architecture, a conservative capability matrix, ADRs, a roadmap, and
  end-to-end use-case guides.
- Six architecture decision records covering the domain/codec-free entity
  graph, the codec trait, borrowed domain views, the Axiolid geometry
  boundary, scaffold-module semantics, and thin facade defaults.
- `openbim-ifc/tests/docs_examples.rs`, which compiles and runs every Rust
  example shown in the documentation so published code cannot drift.
- `scripts/sync-changelog.py`, generating the documentation changelog page
  from this file; `scripts/gate.sh` fails on drift.
- `scripts/check-leakage.py`, rejecting standards material (XSD, PDF,
  `references/`) from the published site.

### Fixed

- `Model::insert` no longer corrupts the type index when it replaces an
  existing entity: the id was appended unconditionally, so re-inserting listed
  it twice under the same type, and replacing an entity with one of a different
  type left it listed under both. `ids_of_type` and `type_histogram` reported
  those duplicates.

### Changed

- Advanced `openbim-step` to `0.2.1` for strict mandatory-header validation,
  line/print-control handling, low-line keywords, and lossless string escapes.
- Delegated generic ISO 10303-21 STEP syntax and ISO 10303-11 EXPRESS parsing
  to `openbim-step`; IFC retains thin model, schema-version, and validation
  adapters.
- Extracted the IFC family from `openbimrs/openbim` into its canonical standalone
  repository while preserving relevant source history.
- Added an independent Cargo workspace, CI workflow, verification gate, project
  documentation, and self-contained regression fixtures.
- Made release-critical package metadata explicit across the nested-workspace
  boundary.

[Unreleased]: https://github.com/openbimrs/ifc/commits/main
