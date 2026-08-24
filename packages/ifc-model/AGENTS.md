# ifc-model instructions

Purpose: Schema-agnostic entity graph and stable ports used by every IFC adapter and projection.

Follow `../AGENTS.md`. Read `PLAN.md` only when assigned implementation or
roadmap work; record progress and blockers there, not here.

## Boundary

Allowed production dependencies: no internal IFC or geometry crate.

## Module ownership

- `codec.rs`: serialization port only
- `id.rs/value.rs`: lossless generic values and typed handles
- `model.rs`: record ownership and basic lookup
- `index.rs`: derived indexes, never domain semantics
- `relation.rs/traverse.rs/spatial.rs`: generic graph queries with explicit budgets
- `guid.rs`: IFC compressed GUID value codec

## Invariants

- Unknown entity and value forms survive codec round trips.
- No schema entity names, material concepts, or geometry types enter this crate.
- Indexes are derived state and must remain coherent across mutation.

Keep `lib.rs` delegating, keep child modules crate-private until they own a real
public contract, and split view/data, traversal, mutation, and validation before
they grow together.

## Verification

Run targeted crate tests and clippy first, then the package architecture/context
gates from `../AGENTS.md`. Record exact exit evidence in `PLAN.md`.
