# ifc-model implementation plan

Status: working core; MODEL-MUT lands transactional authoring. Reverse-index
benchmarks, bounded traversal reuse, provenance and perf baselines remain.
Last updated: 2026-08-19

This is task state, not ambient context. Follow `AGENTS.md`. Claim one task ID,
record blockers here, and check it off only with the stated evidence.

## Established boundary

Schema-agnostic entity graph and stable ports used by every IFC adapter and projection.

## Planned file map

These paths are compiled private scaffold modules. Implement inside the named
owner and expose a public symbol only through an intentional parent re-export.

- `src/index/type_index.rs`: entity-type lookup contract
- `src/index/reverse.rs`: target ID to referring entity/slot index
- `src/index/builder.rs`: coherent initial index construction
- `src/mutation/edit.rs`: explicit edit operations
- `src/mutation/transaction.rs`: validate then commit atomically (implemented)
- `src/mutation/conflict.rs`: conflict and stale-revision diagnostics (implemented)
- `src/traverse/budget.rs`: depth/node/cycle budgets
- `src/traverse/dfs.rs`: bounded depth-first traversal
- `src/traverse/bfs.rs`: bounded breadth-first traversal
- `src/traverse/cycle.rs`: cycle-path reporting
- `src/provenance.rs`: optional source side-table contract

## Work queue

- [ ] `MODEL-INV` - implement and benchmark a coherent reverse-reference index; prove insert/update/remove behavior
  - Evidence: targeted tests plus crate clippy; add a focused fixture/property test.
- [x] `MODEL-MUT` - add transactional authoring operations without domain setters
  - Evidence: targeted tests plus crate clippy; add a focused fixture/property test.
- [ ] `MODEL-TRV` - make traversal budgets and cycle reports reusable by projections
  - Evidence: targeted tests plus crate clippy; add a focused fixture/property test.
- [ ] `MODEL-PRV` - design optional source/provenance side tables without changing Entity
  - Evidence: targeted tests plus crate clippy; add a focused fixture/property test.
- [ ] `MODEL-PERF` - record memory/lookup baselines before changing storage layout
  - Evidence: targeted tests plus crate clippy; add a focused fixture/property test.

## Completion log

Append concise entries as `TASK-ID - proof command/result - material decision`.
Do not paste long logs or transient process state.

MODEL-MUT - cargo test -p ifc-model (17 passing in tests/mutation.rs) - preflight
validates against a PROJECTED model, so a batch may create an entity and
reference it, or remove a target and re-point its referrers, in one unit. Every
failure is decided before the first write, which is why commit needs no undo
log. `Model::revision` bumps on every structural change so a stale transaction
is refused rather than applied to state its author never saw. Removal refuses
to orphan a surviving reference, closing the gap `Model::remove` documents.
