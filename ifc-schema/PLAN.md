# ifc-schema implementation plan

Status: moving generic EXPRESS parsing/model types into
`openbim-step::express`; IFC schema keeps version/profile queries.
Last updated: 2026-08-25

This is task state, not ambient context. Follow `AGENTS.md`. Claim one task ID,
record blockers here, and check it off only with the stated evidence.

## Established boundary

Query IFC schema/version metadata over a generic EXPRESS syntax substrate; it
is metadata, not serialization and not the entity graph.

## Planned file map

These paths already compile as private scaffold owners. Replace a planned-owner
marker with its first real contract and tests; do not add parallel placeholders.

- `src/parser/lexer.rs`: EXPRESS tokens and locations
- `src/parser/declaration.rs`: entity/type/function declarations
- `src/model/inheritance.rs`: subtype closure
- `src/model/attributes.rs`: inherited absolute attributes
- `src/model/rules.rs`: WHERE/UNIQUE/DERIVE/INVERSE metadata

## Work queue

- [ ] `SCHEMA-META` - represent inverse, derived, unique, and WHERE declarations losslessly
  - Evidence: targeted tests plus crate clippy; add a focused fixture/property test.
- [ ] `SCHEMA-DIAG` - attach source spans to parse errors
  - Evidence: targeted tests plus crate clippy; add a focused fixture/property test.
- [ ] `SCHEMA-VERS` - make IFC schema/version profiles explicit
  - Evidence: targeted tests plus crate clippy; add a focused fixture/property test.
- [ ] `SCHEMA-GEN` - generate committed compact manifests for runtime consumers
  - Evidence: targeted tests plus crate clippy; add a focused fixture/property test.
- [ ] `SCHEMA-PERF` - benchmark official IFC2x3/4/4x3 parses before optimizing
  - Evidence: targeted tests plus crate clippy; add a focused fixture/property test.
- [x] `SCHEMA-EXTRACT` - delegate generic EXPRESS parsing/model types without a fork
  - Evidence: official schema regressions, architecture RED/GREEN, standalone gate.

## Completion log

Append concise entries as `TASK-ID - proof command/result - material decision`.
Do not paste long logs or transient process state.

- `SCHEMA-EXTRACT` - parser/AST types now come from `openbim_step::express`;
  all three official IFC schema tests pass through the delegated implementation.
