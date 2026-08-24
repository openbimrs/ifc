# ifc-geometry surface instructions

Scope: Borrowed views and local semantic validation for IFC surface entities.

Follow the crate `../../AGENTS.md`. Read this directory's `PLAN.md` only for assigned
work under parent task(s) `GEOM-SURFACE`. Record progress there.

## Owns

- elementary/swept/bounded/B-spline surface views
- surface parameterization and boundaries
- surface-specific rules

## Does not own

- surface meshing
- surface intersection algorithms
- B-rep topology ownership

## Growth map

`elementary.rs`, `swept.rs`, `bounded.rs`, `bspline.rs`. These source owners already compile as private scaffold modules. Replace a module's planned-owner marker with its first real contract and tests; do not add parallel placeholders.

Every source entity error cites EntityId/type/slot or rule. Add invalid, cycle,
and unsupported cases, not only happy paths.
