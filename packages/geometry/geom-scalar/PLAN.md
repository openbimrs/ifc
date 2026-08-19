# geom-scalar plan

## Done
- Error-free transformations (`two_sum`, `two_diff`, `two_product`).
- `orient2d`: f64 filter with magnitude-scaled error bound, escalating to exact
  expansion arithmetic. Differential-tested against an independent i128 oracle
  and against known naive-f64 failures.

## Goal A: GeometryCompiler (in progress)
Evaluate the three node families `ifc-geometry` actually emits.

- [ ] M1 profile triangulation: Rectangle, Circle, Contour (ear clipping with
      holes via bridge insertion). Orientation certified by `orient2d`.
- [ ] M2 linear extrusion: profile -> closed manifold prism, outward-oriented.
- [ ] M3 transform composition: Instance chains, nested placements.
- [ ] M4 SolidOperation::Boolean -> dispatch to a MeshBoolean provider.
- [ ] M5 `ifc-cli mesh <file.ifc>` end to end.

## Gates
- Wall-minus-openings volume vs Monte-Carlo oracle.
- Manifold in -> manifold out on every fixture that lowers.
- `ifc capabilities` no longer reports "none".

## Deferred
- Revolution (seam handling), SweptDisk, SectionedSpine, BRep, surfaces.
- Reason: `ifc-geometry` does not emit them yet; adding them before there is a
  producer would be untested speculation.
