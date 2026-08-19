# ifc-geometry rules instructions

Scope: Actionable IFC geometry WHERE-rule validation with stable violations.

Follow the crate `../../AGENTS.md`. Read this directory's `PLAN.md` only for assigned
work under parent task(s) `GEOM-CENSUS`. Record progress there.

## Owns

- rule registry and typed violations
- local dimensionality/degeneracy/agreement rules
- valid-case pair for every rejection

## Does not own

- reimplementing parser type constraints
- kernel numerical validation
- claiming unsupported rules passed

## Growth map

`violation.rs`, `placement.rs`, `solid.rs`. These source owners already compile as private scaffold modules. Replace a module's planned-owner marker with its first real contract and tests; do not add parallel placeholders.

Every source entity error cites EntityId/type/slot or rule. Add invalid, cycle,
and unsupported cases, not only happy paths.
