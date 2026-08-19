# ifc-geometry constraint instructions

Scope: Views and resolution inputs for placements, grids, and geometric connections.

Follow the crate `../../AGENTS.md`. Read this directory's `PLAN.md` only for assigned
work under parent task(s) `GEOM-PLACE`. Record progress there.

## Owns

- local/grid/linear placement references
- grid axes/intersections
- connection geometry references
- cycle-detection inputs and local rules

## Does not own

- global spatial indexing
- product semantics
- map/CRS conversion

## Growth map

`local.rs`, `grid.rs`, `connection.rs`. These source owners already compile as private scaffold modules. Replace a module's planned-owner marker with its first real contract and tests; do not add parallel placeholders.

Every source entity error cites EntityId/type/slot or rule. Add invalid, cycle,
and unsupported cases, not only happy paths.
