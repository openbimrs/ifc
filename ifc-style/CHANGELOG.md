# Changelog -- ifc-style

All notable changes to the `ifc-style` crate are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate follows Semantic Versioning independently of its siblings:
a release here does not imply a release of any other crate in the family.

Changes up to and including 0.2.0 are recorded family-wide in the
[repository changelog](../CHANGELOG.md); that file is the archive for
everything released before per-crate changelogs began.

## [Unreleased]

### Added

- `IndexedTextureMap::triangle_coordinates(triangle_count)` resolves an
  `IfcIndexedTriangleTextureMap` to `(s, t)` coordinates for each triangle
  corner, in `CoordIndex` order. Corners, not vertices: one position may
  carry a different coordinate in each triangle that uses it. A shorter
  `TexCoordIndex` covers only the leading triangles, a longer one is an
  error, and an omitted one returns `None` because the schema does not
  define it (#30).

### Fixed

- **Breaking:** `IndexedTextureMap::maps` now returns `Vec<EntityId>`.
  `Maps` is `LIST [1:?] OF IfcSurfaceTexture`, but it was read as a single
  reference, so it returned an error on every conforming file, including
  those this crate writes itself.

## [0.2.0] - 2026-09-22

First release under per-crate versioning. See the
[repository changelog](../CHANGELOG.md) for the family-wide history
that produced this version.

[Unreleased]: https://github.com/openbimrs/ifc/compare/ifc-style-v0.2.0...HEAD
[0.2.0]: https://github.com/openbimrs/ifc/releases/tag/v0.2.0
