# geom-compile plan

## Done
- Profile flattening: rectangle (solid + hollow), circle (disk + annulus),
  with an explicit chord budget and a segment count derived from the sagitta.
- Triangulation with holes via `earcut` (ADR 0015).
- Linear extrusion: caps from the triangulation, sides per boundary loop.
- Gates: exact volume, directed-edge manifoldness, disk convergence from below,
  differential oracle vs `geom-scalar`, and end-to-end acceptance by
  `geom-boolmesh`.

## Next
- `GeometryCompiler` impl: walk `GeometryGraph`, handle `Profile`,
  `SolidOperation::Extrusion`, `Instance` transforms, and
  `SolidOperation::Boolean` dispatch.
- Wire `ifc-cli mesh <file.ifc>`.
- Validate the fixture corpus: manifold-in/manifold-out on all 11 fixtures.

## Deferred
- Rectangle corner radii (`outer_radius`, `inner_radius`) currently ignored;
  profiles render with sharp corners.
- Revolution, swept disk, sectioned spine: seam handling needs care, see the
  Goal A risk note.
- Ellipse and structural section profiles.
