# ifc-cost implementation plan

Status: working reference domain projection; authoring and richer units/rates incomplete.
Last updated: 2026-08-19

This is task state, not ambient context. Follow `AGENTS.md`. Claim one task ID,
record blockers here, and check it off only with the stated evidence.

## Established boundary

Borrowed cost-item, quantity, rate, and amount projections over ifc-model.

## Planned file map

These paths already compile as private scaffold owners. Replace a planned-owner
marker with its first real contract and tests; do not add parallel placeholders.

- `src/rate.rs`: cost value/rate semantics
- `src/assignment.rs`: schedule/control/product associations
- `src/mutation.rs`: transactional authoring through ifc-model ports

## Work queue

- [ ] `COST-RATE` - complete rate/category/formula projections
  - Evidence: targeted tests plus crate clippy; add a focused fixture/property test.
- [ ] `COST-REL` - resolve cost item nesting and assignments with cycle budgets
  - Evidence: targeted tests plus crate clippy; add a focused fixture/property test.
- [ ] `COST-MUT` - add authoring only after MODEL-MUT exists
  - Evidence: targeted tests plus crate clippy; add a focused fixture/property test.
- [ ] `COST-UNIT` - make currency/unit mismatch explicit
  - Evidence: targeted tests plus crate clippy; add a focused fixture/property test.
- [ ] `COST-CORPUS` - add real-file semantic fixtures beyond the codec roundtrip proof
  - Evidence: targeted tests plus crate clippy; add a focused fixture/property test.

## Completion log

Append concise entries as `TASK-ID - proof command/result - material decision`.
Do not paste long logs or transient process state.
