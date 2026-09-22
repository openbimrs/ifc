# ifc-control implementation plan

Status: the four owned `IfcControl` subtypes are authored and locally gated.
Last updated: 2026-09-22

This is task state, not ambient context. Follow `AGENTS.md`; claim one task ID,
record blockers/decisions under it, and check it off only with evidence.

## Bounded contract

Transaction-staged authoring for `IfcPermit`, `IfcProjectOrder`,
`IfcActionRequest`, and `IfcPerformanceHistory`. This does not imply issuing,
approval workflow, expiry, policy evaluation, or the assignment relationships
that attach a control to the work it governs.

`IfcCostItem`, `IfcCostSchedule`, `IfcWorkCalendar` and `IfcWorkControl` are
also `IfcControl` subtypes but belong to `ifc-cost` and `ifc-schedule`. The
split follows the domain, not the supertype; do not move them here.

`IfcWorkOrder` is not an IFC entity. A work order is an `IfcProjectOrder`
whose `PredefinedType` is `WORKORDER`.

## Planned file map

- `src/authoring.rs`: `ControlKind`, typed drafts, and transaction staging
- `src/error.rs`: typed authoring refusals
- `tests/control.rs`: slot layout, refusals, and two-schema agreement

## Work queue

- [x] `CONTROL-WRITE` - schema-resolved staging for the four owned subtypes
- [x] `CONTROL-ENUM` - per-entity predefined-type tokens and USERDEFINED fallback
- [x] `CONTROL-TAIL` - `IfcPerformanceHistory`'s divergent slots 6-8
- [x] `CONTROL-PROOF` - mutation sweep, facade wiring, docs, and full gate
- [ ] `CONTROL-READ` - borrowed projections, added when a consumer needs them
- [ ] `CONTROL-ASSIGN` - `IfcRelAssignsToControl`, once relationship ownership is settled

## Completion log

- `CONTROL-WRITE/ENUM/TAIL` - 8 focused public tests cover declared slot
  layout, foreign-enum refusal, USERDEFINED without `ObjectType`, the
  performance-history tail in both directions, and identity validation.
- Arity is read from the schema tables rather than hardcoded; the two-schema
  test asserts IFC4 and IFC4X3 agree on arity, slot 5, and every enum token.
- Mutation proof - 10/10 killed on a clean baseline, including the
  predefined-type slot shift and the upper-case type-name convention.
- Boundary proof - all eight `IfcControl` subtypes resolve to exactly one
  owning crate with no overlap.
