# ifc-step instructions

Purpose: ISO 10303-21 STEP codec adapter between bytes/files and ifc-model.

Follow `../AGENTS.md`. Read `PLAN.md` only when assigned implementation or
roadmap work; record progress and blockers there, not here.

## Boundary

Allowed production dependencies: ifc-model only in production; syntax helpers and performance libraries are private implementation details.

## Module ownership

- `lexer.rs/escape.rs`: byte syntax and STEP string escapes
- `parser.rs/header.rs`: syntax records to Model
- `writer.rs`: deterministic Model serialization
- `partition.rs`: record-aligned partition discovery only
- `error.rs`: syntax and I/O failures

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
