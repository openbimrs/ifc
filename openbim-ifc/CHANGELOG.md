# Changelog -- openbim-ifc

All notable changes to the `openbim-ifc` crate are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate follows Semantic Versioning independently of its siblings:
a release here does not imply a release of any other crate in the family.

Changes up to and including 0.2.0 are recorded family-wide in the
[repository changelog](../CHANGELOG.md); that file is the archive for
everything released before per-crate changelogs began.

## [Unreleased]

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

[Unreleased]: https://github.com/openbimrs/ifc/compare/openbim-ifc-v0.4.0...HEAD
[0.4.0]: https://github.com/openbimrs/ifc/releases/tag/openbim-ifc-v0.4.0
[0.3.1]: https://github.com/openbimrs/ifc/releases/tag/openbim-ifc-v0.3.1
[0.3.0]: https://github.com/openbimrs/ifc/releases/tag/openbim-ifc-v0.3.0
[0.2.0]: https://github.com/openbimrs/ifc/releases/tag/v0.2.0
