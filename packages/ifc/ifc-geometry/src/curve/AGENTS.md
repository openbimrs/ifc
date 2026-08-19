# ifc-geometry curve instructions

Scope: Borrowed views and local semantic validation for IFC curve entities.

Follow the crate `../../AGENTS.md`. Read this directory's `PLAN.md` only for assigned
work under parent task(s) `GEOM-CURVE`. Record progress there.

## Owns

- line/polyline/conic/trimmed/composite/offset/B-spline views
- curve-specific WHERE-rule inputs
- trim/select semantics without graph construction

## Does not own

- tessellation or tolerance policy
- placement-chain resolution
- surface/solid lowering

## Growth map

`line.rs`, `polyline.rs`, `conic.rs`, `trimmed.rs`, `composite.rs`, `offset.rs`, `bspline.rs`. These source owners already compile as private scaffold modules. Replace a module's planned-owner marker with its first real contract and tests; do not add parallel placeholders.

Every source entity error cites EntityId/type/slot or rule. Add invalid, cycle,
and unsupported cases, not only happy paths.
