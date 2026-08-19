# geom-scalar plan

## Done

- Error-free transformations: `two_sum`, `two_diff`, `two_product`.
- `orient2d` with filter -> exact escalation, certified results.

## Next

- `orient3d`: 3x3 determinant, same cascade shape.
- `incircle` / `insphere`: needed by Delaunay and by robust mesh repair.
- Static filters: precompute bounds when coordinate ranges are known, to skip
  the magnitude computation per call.

## Deliberately absent

- SIMD, threading, GPU. This crate is the oracle; optimized paths live in
  `geom-backend-*` and are validated against it.
