# ifc-resource implementation plan

Status: bounded IFC4 construction-resource slice complete: occurrences, actors,
resource types, inventory, and usage quantities are all implemented.
Cross-version (IFC2X3/IFC4X3), scheduling, costing, and simulation behavior
remain explicitly out of scope.
Last updated: 2026-09-01

This is task state, not ambient context. Follow `AGENTS.md`; claim one task ID,
record blockers/decisions under it, and check it off only with evidence.

## Established boundary

Borrowed IFC4 construction-resource occurrence, actor (person/organization/role),
resource-type, inventory, authored resource-time, usage-quantity, allocation, and
bounded nesting semantics are implemented. Cross-version (IFC2X3/IFC4X3),
scheduling, costing, and simulation behavior remain outside this crate; they
compose at the facade/application layer against other domain crates.

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

- [x] `RES-ACTOR` - implement actor/organization/role projections
  - Evidence: 6 focused view/query tests (identity/name-set rules, WR1
    USERDEFINED rule, relationship membership, wrong-reference-type refusal),
    strict crate Clippy.
- [x] `RES-BASE` - implement construction-resource occurrences and bounded nesting
  - Evidence: strict projections, authored-order traversal, malformed graph tests, and budgets.
- [x] `RES-SPECIAL` - classify six concrete occurrence kinds and validate predefined types
  - Evidence: schema slot contract and labor/equipment/crew projection tests.
- [x] `RES-TYPE` - prove and implement construction-resource type projections
  - Evidence: 6 focused tests covering all six concrete type kinds, WR1
    USERDEFINED-requires-ResourceType rule, `IfcRelDefinesByType` assignment
    resolution, and duplicate-assignment-with-different-type refusal; strict
    crate Clippy.
- [x] `RES-INV` - implement inventory projections
  - Evidence: 5 focused tests covering metadata/jurisdiction/responsible-persons
    projection, `IfcActorSelect` wrong-reference-type refusal, and
    `IfcRelAssignsToGroup` membership resolution with authored order and
    unrelated-group filtering; strict crate Clippy.
- [x] `RES-TIME` - implement authored `IfcResourceTime` values
  - Evidence: finite positive ratio, text/date/duration, and reference-type tests.
- [x] `RES-USAGE` - implement usage quantity semantics
  - Evidence: 5 focused tests covering all six simple-quantity kinds via a
    representative subset, non-negativity/finite-value refusal,
    `IfcPhysicalComplexQuantity` member resolution and self-reference refusal,
    and `IfcConstructionResource.BaseQuantity` resolution; strict crate Clippy.
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
- `RES-ACTOR` - `cargo test -p ifc-resource --test actor` (6/6 passing), strict Clippy/rustdoc, 3/3 killed semantic mutants (WR1 guard, `IdentifiablePerson`, `ValidSetOfNames`), restored GREEN - `IfcPerson`/`IfcOrganization`/`IfcActorRole`/`IfcPersonAndOrganization` project borrowed fields and enforce `IdentifiablePerson`, `ValidSetOfNames`, and `WR1` without claiming actor-assignment or address-detail semantics.
- `RES-TYPE` - `cargo test -p ifc-resource --test resource_type` (6/6 passing), strict Clippy/rustdoc, 3/3 killed semantic mutants (USERDEFINED guard, related-object filter, duplicate-type refusal), restored GREEN - all six `IfcConstructionResourceType` subtypes classify by schema type name; `assigned_resource_type` resolves `IfcRelDefinesByType`, accepts repeated relations naming the same type, and refuses a second relation naming a different type for the same occurrence.
- `RES-INV` - `cargo test -p ifc-resource --test inventory` (5/5 passing), strict Clippy/rustdoc, 1/1 killed semantic mutant (relating-group filter), restored GREEN - `IfcInventory` projects metadata and `IfcActorSelect` jurisdiction; membership resolves via `IfcRelAssignsToGroup` in authored order and ignores relations naming a different group.
- `RES-USAGE` - `cargo test -p ifc-resource --test usage_quantity` (5/5 passing), strict Clippy/rustdoc, 2/2 killed semantic mutants (non-negativity guard, `NoSelfReference`), restored GREEN - `IfcPhysicalSimpleQuantity` subtypes project a typed value and enforce the shared non-negative/finite rule; `IfcPhysicalComplexQuantity` resolves ordered members and enforces `NoSelfReference`.
