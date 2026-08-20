# geom-compile plan

## Done
- Profile flattening (rectangle/circle/ellipse + hollow variants, contours).
- `earcut` triangulation with holes; certified oracle differential gate.
- Linear extrusion: caps + sides, outward winding, edge-manifold gate.
- `ScalarCompiler`: iterative post-order DAG walk, memoised, generic over
  the boolean provider. Handles TriMesh, Instance, Collection, Extrusion,
  Boolean. Every other family returns `Unsupported` with the named capability.

## Next
- Revolution (care on the seam: shared vertices at 0 and 2pi).
- `BoundedHalfSpace` for `IfcBooleanClippingResult`.
- Tapered extrusion.

## Invariants proven by mutation
- Memoisation is load-bearing (removal kills 7 tests).
- Mirror placements keep outward winding (negative determinant reverses).
- `append_mesh` rebases indices; the gate uses DIFFERENT-sized solids because
  two identical cubes sum to the same volume either way.
- Unsupported nodes name the capability they need, asserted exactly.
- Foreign graph handles are refused, never silently indexed.
