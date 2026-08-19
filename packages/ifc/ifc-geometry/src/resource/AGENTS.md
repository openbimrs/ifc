# ifc-geometry resource instructions

Scope: Zero-copy views for IfcGeometryResource values and helper functions.

Follow the crate `../../AGENTS.md`. Read this directory's `PLAN.md` only for assigned
work under parent task(s) `GEOM-CENSUS`. Record progress there.

## Owns

- points/directions/axes/placements/operators
- representation maps and geometric helper functions
- absolute STEP slot constants and accessor errors

## Does not own

- recursive model traversal
- unit conversion or transform composition
- kernel/graph types

## Growth map

`point.rs`, `direction.rs`, `axes.rs`, `placement.rs`, `operator.rs`, `mapped.rs`, `functions.rs`. These source owners already compile as private scaffold modules. Replace a module's planned-owner marker with its first real contract and tests; do not add parallel placeholders.

Every source entity error cites EntityId/type/slot or rule. Add invalid, cycle,
and unsupported cases, not only happy paths.
