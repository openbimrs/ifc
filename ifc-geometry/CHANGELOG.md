# Changelog -- ifc-geometry

All notable changes to the `ifc-geometry` crate are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate follows Semantic Versioning independently of its siblings:
a release here does not imply a release of any other crate in the family.

Changes up to and including 0.2.0 are recorded family-wide in the
[repository changelog](../CHANGELOG.md); that file is the archive for
everything released before per-crate changelogs began.

## [Unreleased]

## [0.3.0] - 2026-09-23

### Added

- Lowering an `IfcTriangulatedFaceSet` now attaches texture coordinates from
  its `IfcIndexedTriangleTextureMap` as a corner-indexed attribute channel
  named `uv` (`lower::tessellated::UV_CHANNEL`), one `(s, t)` per triangle
  corner (#30). Positions stay shared, so the mesh stays closed. A shorter
  `TexCoordIndex` leaves trailing triangles `UNMAPPED`; an omitted one adds
  no channel; a longer one, or an index outside the texture vertices, is an
  error. A second map on the same face set becomes `uv1`, and so on.

### Known limits

- `IfcIndexedPolygonalTextureMap` (IFC4X3) is not lowered.

### Changed

- **Breaking:** requires Axiolid 0.3 (`axiolid-mesh` 0.3 adds
  `AttributeChannel::corner_indices`).

## [0.2.0] - 2026-09-22

First release under per-crate versioning. See the
[repository changelog](../CHANGELOG.md) for the family-wide history
that produced this version.

[Unreleased]: https://github.com/openbimrs/ifc/compare/ifc-geometry-v0.3.0...HEAD
[0.3.0]: https://github.com/openbimrs/ifc/releases/tag/ifc-geometry-v0.3.0
[0.2.0]: https://github.com/openbimrs/ifc/releases/tag/v0.2.0
