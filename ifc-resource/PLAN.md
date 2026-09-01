# ifc-resource implementation plan

Status: bounded IFC4 construction-resource slice implemented; broader resource families remain open.
Last updated: 2026-09-01

This is task state, not ambient context. Follow `AGENTS.md`; claim one task ID,
record blockers/decisions under it, and check it off only with evidence.

## Established boundary

Borrowed IFC4 construction-resource occurrence, authored resource-time, allocation,
and bounded nesting semantics are implemented. Actor, inventory, resource-type,
usage-quantity, cross-version, scheduling, costing, and simulation behavior remain
outside the completed slice.

## Planned file map

These paths are compiled private scaffold modules or implemented capability seams.
Implement inside the named owner and expose a public symbol only through an
intentional parent re-export.

- `src/actor/person.rs`: people and identities
- `src/actor/organization.rs`: organizations/relationships
- `src/actor/role.rs`: actor roles
- `src/resource/base.rs`: construction resource occurrences
- `src/resource/type.rs`: resource types
- `src/resource/nesting.rs`: resource composition
- `src/labour/resource.rs`: labor resources
- `src/equipment/resource.rs`: equipment resources
- `src/crew/resource.rs`: crews
- `src/inventory/definition.rs`: inventory metadata
- `src/inventory/items.rs`: contained asset links
- `src/usage/time.rs`: authored resource time
- `src/usage/quantity.rs`: usage quantities
- `src/query/allocation.rs`: assignment queries

## Work queue

- [ ] `RES-ACTOR` - implement actor/organization/role projections
  - Evidence: focused view/query tests, invalid/cycle cases, and crate clippy.
- [x] `RES-BASE` - implement construction-resource occurrences and bounded nesting
  - Evidence: strict projections, authored-order traversal, malformed graph tests, and budgets.
- [x] `RES-SPECIAL` - classify six concrete occurrence kinds and validate predefined types
  - Evidence: schema slot contract and labor/equipment/crew projection tests.
- [ ] `RES-TYPE` - prove and implement construction-resource type projections
  - Evidence: official schema comparison, focused type/select tests, and crate clippy.
- [ ] `RES-INV` - implement inventory projections
  - Evidence: focused view/query tests, invalid/cycle cases, and crate clippy.
- [x] `RES-TIME` - implement authored `IfcResourceTime` values
  - Evidence: finite positive ratio, text/date/duration, and reference-type tests.
- [ ] `RES-USAGE` - implement usage quantity semantics
  - Evidence: focused projection/validation tests and crate clippy.
- [x] `RES-QUERY` - resolve authored allocations without schedule/cost coupling
  - Evidence: SELECT, SET, self-reference, ordering, and dangling/type tests.
- [x] `RES-AUTH` - transaction-stage selected resource/time/relation authoring
  - Evidence: round-trip, rejection atomicity, facade consumer, and STEP write/read tests.

## Completion log

Append concise entries as `TASK-ID - proof command/result - material decision`.
Do not paste long logs or duplicate standing rules from `AGENTS.md`.

- `RES-BASE` - 13 focused crate tests plus 20/20 semantic mutants - preserve authored LIST order and refuse malformed/cyclic/budget-exceeding composition.
- `RES-SPECIAL` - bundled IFC4 inherited-slot assertions and enum tests - project all six occurrence kinds without claiming type-resource support.
- `RES-TIME` - malformed scalar/reference tests - preserve authored strings and finite positive ratios without calendar evaluation.
- `RES-QUERY` - allocation and composition query tests - validate SELECT/SET/list semantics and `IfcRelAssigns.WR1` object-category matching with deterministic model/authored order.
- `RES-AUTH` - strict Clippy, facade STEP round trip, and external minimal-feature consumer - validate before one transaction and preserve model length/revision on rejection.
