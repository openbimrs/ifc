# Changelog -- ifc-geometry

All notable changes to the `ifc-geometry` crate are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate follows Semantic Versioning independently of its siblings:
a release here does not imply a release of any other crate in the family.

Changes up to and including 0.2.0 are recorded family-wide in the
[repository changelog](../CHANGELOG.md); that file is the archive for
everything released before per-crate changelogs began.

## [Unreleased]

### Fixed

- `IfcPolygonalBoundedHalfSpace` clips now compile (#45). `PolygonalBoundary`
  was lowered through the 3D curve path as a `Curve3`, but Axiolid's
  `BoundedHalfSpace` contract and reference compiler require a `Curve2`, so
  every such `IfcBooleanClippingResult` was refused with `half-space boundary
  .. is not a Curve2 node` although lowering succeeded. The boundary now
  lowers as a `Curve2` polyline in `Position`'s XY plane, lengths converted
  to metres. A 3D boundary point is accepted only with `z = 0`; any other `z`
  violates `BoundaryDim` and is refused as `Degenerate`, naming the point,
  instead of being projected.

### Known limits

- An `IfcCompositeCurve` or `IfcIndexedPolyCurve` boundary is refused as
  `Unsupported` rather than lowered (#43).
- With `axiolid-construct` 0.3.0 the compiled clip ignores the in-plane
  translation of `Position`: the boundary is placed at the base plane's
  origin, with no error. Lowering carries the translation correctly; the fix
  is axiolid/kernel#164. `tests/bounded_halfspace_compile.rs` pins it with
  an ignored test that passes against the fixed kernel.

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
