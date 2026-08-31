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
- [x] `SCHEMA-GEN` - generate committed compact manifests for runtime consumers
  - Evidence: `Schema::ifc4()` bundles a compiled 120178-byte artifact
    (776 entities, 397 types) built via `cargo run -p ifc-schema --features
    generation --bin ifc-schema-generate -- <path to IFC4.exp>`; 15 crate
    tests pass under `--features generation` (default `cargo test -p
    ifc-schema` runs 13, excluding the generation-only round-trip cases);
    `ifc-author/tests/real_schema.rs` now sources the bundled schema by
    default and cross-checks it against a raw `references/ifc-spec` parse
    when present, closing openbimrs/ifc#4.
- [x] `SCHEMA-4X3-BUNDLE` - generate, bundle, and route canonical IFC4X3 ADD2 tables
  - Evidence: committed 137179-byte artifact; 876 entities and 436 `TYPE ` declarations; constructor/cache/version dispatch and real-source layout tests; archive leakage, mutation, and full gates.
- [ ] `SCHEMA-PERF` - benchmark official IFC2x3/4/4x3 parses before optimizing
  - Evidence: targeted tests plus crate clippy; add a focused fixture/property test.
- [x] `SCHEMA-EXTRACT` - delegate generic EXPRESS parsing/model types without a fork
  - Evidence: official schema regressions, architecture RED/GREEN, standalone gate.

## Completion log

Append concise entries as `TASK-ID - proof command/result - material decision`.
Do not paste long logs or transient process state.

- `SCHEMA-EXTRACT` - parser/AST types now come from `openbim_step::express`;
  all three official IFC schema tests pass through the delegated implementation.
