# ifc-step implementation plan

Status: extracting generic Part 21 syntax into `openbim-step`; this crate
retains only the IFC model adapter.
Last updated: 2026-08-25

This is task state, not ambient context. Follow `AGENTS.md`. Claim one task ID,
record blockers here, and check it off only with the stated evidence.

## Established boundary

IFC adapter between generic `openbim-step` exchange structures and
`ifc-model`. Generic ISO 10303-21 syntax belongs below this repository.

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
- [x] `STEP-BUDGET` - bound recursive aggregate/typed-value parsing
  - Evidence: targeted tests plus crate clippy; add a focused fixture/property test.
- [ ] `STEP-PAR` - wire partitioned parsing only after differential correctness tests
  - Evidence: targeted tests plus crate clippy; add a focused fixture/property test.
- [x] `STEP-WRITE` - prove deterministic ordering and numeric/string edge cases
  - Evidence: 18 crate tests/doc-tests, strict crate clippy, and 3/3 focused
    semantic mutants killed; repeated writes are byte-identical in model/file
    order, finite real extremes and signed zero round-trip bit-exactly,
    text/binary/typed values survive, and nested non-finite reals are refused
    with entity/slot context.
- [ ] `STEP-PERF` - establish mmap/read/parse/write benchmark baselines
  - Evidence: targeted tests plus crate clippy; add a focused fixture/property test.
- [x] `STEP-EXTRACT` - consume generic STEP syntax without retaining a fork
  - Evidence: architecture RED/GREEN, fixture round trips, standalone gate.

## Completion log

Append concise entries as `TASK-ID - proof command/result - material decision`.
Do not paste long logs or transient process state.

- `STEP-ORPH` - removed four uncompiled files containing a duplicate model and
  `unimplemented!()` reader; workspace module-reachability gate passes.
- `STEP-BUDGET` - generic parser and writer reject nesting above 128; the
  limit regression was mutation-probed by changing it to 129 and observing exit 101.
- `STEP-EXTRACT` - `ifc-step` now contains only IFC graph/header/value conversion;
  fixture parse and semantic round-trip tests pass against local `openbim-step`.
- `STEP-WRITE` - `cargo +1.88.0 test -p ifc-step` plus strict crate clippy;
  output is deterministic in model order, finite IEEE-754 edge values preserve
  their value and signed zero, and non-finite values fail before serialization.
