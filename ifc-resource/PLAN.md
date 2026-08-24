# ifc-resource implementation plan

Status: architecture scaffold; resource families remain to implement.
Last updated: 2026-08-19

This is task state, not ambient context. Follow `AGENTS.md`; claim one task ID,
record blockers/decisions under it, and check it off only with evidence.

## Established boundary

Borrowed construction resource, actor, inventory, crew, equipment, labour, and usage projections.

## Planned file map

These paths are compiled private scaffold modules. Implement inside the named
owner and expose a public symbol only through an intentional parent re-export.

- `src/actor/person.rs`: people and identities
- `src/actor/organization.rs`: organizations/relationships
- `src/actor/role.rs`: actor roles
- `src/resource/base.rs`: construction resource base
- `src/resource/type.rs`: resource types
- `src/resource/nesting.rs`: resource composition
- `src/labour/resource.rs`: labor resources
- `src/equipment/resource.rs`: equipment resources
- `src/crew/resource.rs`: crews
- `src/inventory/definition.rs`: inventory metadata
- `src/inventory/items.rs`: contained asset links
- `src/usage/time.rs`: usage time
- `src/usage/quantity.rs`: usage quantities
- `src/query/allocation.rs`: assignment queries

## Work queue

- [ ] `RES-ACTOR` - implement actor/organization/role projections
  - Evidence: focused view/query tests, invalid/cycle cases, and crate clippy.
- [ ] `RES-BASE` - implement construction resources, types, and nesting
  - Evidence: focused view/query tests, invalid/cycle cases, and crate clippy.
- [ ] `RES-SPECIAL` - implement labor/equipment/crew specializations
  - Evidence: focused view/query tests, invalid/cycle cases, and crate clippy.
- [ ] `RES-INV` - implement inventory projections
  - Evidence: focused view/query tests, invalid/cycle cases, and crate clippy.
- [ ] `RES-USAGE` - implement usage time/quantity semantics
  - Evidence: focused view/query tests, invalid/cycle cases, and crate clippy.
- [ ] `RES-QUERY` - resolve allocations without schedule/cost crate coupling
  - Evidence: focused view/query tests, invalid/cycle cases, and crate clippy.

## Completion log

Append concise entries as `TASK-ID - proof command/result - material decision`.
Do not paste long logs or duplicate standing rules from `AGENTS.md`.
