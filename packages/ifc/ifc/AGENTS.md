# ifc instructions

Purpose: Feature-gated convenience facade; it owns no records, semantics, codec, or geometry implementation.

Follow `../AGENTS.md`. Read `PLAN.md` only when assigned implementation or
roadmap work; record progress and blockers there, not here.

## Boundary

Allowed production dependencies: only explicitly feature-selected IFC crates; no direct geometry backend.

## Module ownership

- `lib.rs`: feature declarations and deliberate re-exports only

## Invariants

- No default feature silently enables heavy domain or geometry capability.
- Every feature builds in isolation and all combinations preserve dependency boundaries.
- Application workflows do not migrate into the facade.

Keep `lib.rs` delegating, keep child modules crate-private until they own a real
public contract, and split view/data, traversal, mutation, and validation before
they grow together.

## Verification

Run targeted crate tests and clippy first, then the package architecture/context
gates from `../AGENTS.md`. Record exact exit evidence in `PLAN.md`.
