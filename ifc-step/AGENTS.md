# ifc-step instructions

Purpose: ISO 10303-21 STEP codec adapter between bytes/files and ifc-model.

Follow `../AGENTS.md`. Read `PLAN.md` only when assigned implementation or
roadmap work; record progress and blockers there, not here.

## Boundary

Allowed production dependencies: `ifc-model` and the generic
`openbim-step` substrate. This crate must not implement or fork generic
ISO 10303-21 syntax.

## Module ownership

- `parser.rs`: generic STEP records/parameters to `ifc-model`
- `writer.rs`: `ifc-model` to generic STEP records/parameters
- `error.rs`: IFC adapter error mapping

Lexer/tokenizer, escapes, headers/sections, generic records/parameters,
partitioning, source spans, syntax diagnostics, and event sinks belong to
`openbim-step`, never here.

## Invariants

- Parse syntax, never entity semantics.
- Trust command exit status; codec round-trip proof compares entity graphs, not normalized bytes.
- Parallel parsing is not claimed until it is used and benchmarked.

Keep `lib.rs` delegating, keep child modules crate-private until they own a real
public contract, and split view/data, traversal, mutation, and validation before
they grow together.

## Verification

Run targeted crate tests and clippy first, then the package architecture/context
gates from `../AGENTS.md`. Record exact exit evidence in `PLAN.md`.
