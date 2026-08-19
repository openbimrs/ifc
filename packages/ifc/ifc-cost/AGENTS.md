# ifc-cost instructions

Purpose: Borrowed cost-item, quantity, rate, and amount projections over ifc-model.

Follow `../AGENTS.md`. Read `PLAN.md` only when assigned implementation or
roadmap work; record progress and blockers there, not here.

## Boundary

Allowed production dependencies: ifc-model only.

## Module ownership

- `view.rs`: typed entity views and absolute slots
- `item.rs`: cost item relationships
- `amount.rs`: monetary/quantity calculations over supplied values
- `query.rs`: bounded model lookups
- `error.rs`: semantic projection failures

## Invariants

- The model owns storage; views borrow.
- Money/unit arithmetic must not silently mix currencies or dimensions.
- Geometry-derived quantities are supplied by orchestration, never computed here.

Keep `lib.rs` delegating, keep child modules crate-private until they own a real
public contract, and split view/data, traversal, mutation, and validation before
they grow together.

## Verification

Run targeted crate tests and clippy first, then the package architecture/context
gates from `../AGENTS.md`. Record exact exit evidence in `PLAN.md`.
