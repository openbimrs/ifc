# ifc-geometry select instructions

Scope: Compiled EXPRESS SELECT membership and subtype classification.

Follow the crate `../../AGENTS.md`. Read this directory's `PLAN.md` only for assigned
work under parent task(s) `GEOM-CENSUS`. Record progress there.

## Owns

- entity and aggregate select projections
- compiled subtype closure used by selects
- positive and negative membership tests

## Does not own

- schema parsing at runtime
- lowering decisions beyond declared membership
- wildcard acceptance of unknown types

## Growth map

`entity_selects.rs`, `aggregate_selects.rs`, `subtype.rs`. These source owners already compile as private scaffold modules. Replace a module's planned-owner marker with its first real contract and tests; do not add parallel placeholders.

Every source entity error cites EntityId/type/slot or rule. Add invalid, cycle,
and unsupported cases, not only happy paths.
