# ifc-step implementation plan

Status: working STEP reader/writer on fixture corpus; historical orphan scaffold files removed.
Last updated: 2026-08-19

This is task state, not ambient context. Follow `AGENTS.md`. Claim one task ID,
record blockers here, and check it off only with the stated evidence.

## Established boundary

ISO 10303-21 STEP codec adapter between bytes/files and ifc-model.

## Planned file map

These paths already compile as private scaffold owners. Replace a planned-owner
marker with its first real contract and tests; do not add parallel placeholders.

- `src/parser/record.rs`: one DATA record parser if parser.rs reaches split threshold
- `src/parser/value.rs`: recursive value parser with budget
- `src/writer/value.rs`: value formatting
- `benches/codec.rs`: throughput and allocation baseline

## Work queue

- [x] `STEP-ORPH` - delete or deliberately integrate stale reader.rs/resolve.rs/scan.rs/value.rs; duplicate models must not survive
  - Evidence: targeted tests plus crate clippy; add a focused fixture/property test.
- [ ] `STEP-BUDGET` - bound recursive aggregate/typed-value parsing
  - Evidence: targeted tests plus crate clippy; add a focused fixture/property test.
- [ ] `STEP-PAR` - wire partitioned parsing only after differential correctness tests
  - Evidence: targeted tests plus crate clippy; add a focused fixture/property test.
- [ ] `STEP-WRITE` - prove deterministic ordering and numeric/string edge cases
  - Evidence: targeted tests plus crate clippy; add a focused fixture/property test.
- [ ] `STEP-PERF` - establish mmap/read/parse/write benchmark baselines
  - Evidence: targeted tests plus crate clippy; add a focused fixture/property test.

## Completion log

Append concise entries as `TASK-ID - proof command/result - material decision`.
Do not paste long logs or transient process state.

- `STEP-ORPH` - removed four uncompiled files containing a duplicate model and
  `unimplemented!()` reader; workspace module-reachability gate passes.
