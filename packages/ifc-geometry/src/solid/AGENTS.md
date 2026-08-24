# ifc-geometry solid instructions

Scope: Borrowed views and local semantic validation for IFC solid/model entities.

Follow the crate `../../AGENTS.md`. Read this directory's `PLAN.md` only for assigned
work under parent task(s) `GEOM-BREP, GEOM-SOLID`. Record progress there.

## Owns

- swept/CSG/boolean/half-space/B-rep/tessellated/surface-model/bbox views
- operand/select classification
- solid-specific WHERE rules

## Does not own

- boolean evaluation
- triangulation or healing
- recursive lowering/graph ownership

## Growth map

`swept/`, `csg.rs`, `boolean.rs`, `halfspace.rs`, `brep.rs`, `tessellated/`, `surface_model.rs`, `bbox.rs`. These source owners already compile as private scaffold modules. Replace a module's planned-owner marker with its first real contract and tests; do not add parallel placeholders.

Every source entity error cites EntityId/type/slot or rule. Add invalid, cycle,
and unsupported cases, not only happy paths.
