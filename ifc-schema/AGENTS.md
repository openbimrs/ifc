# ifc-schema instructions

Purpose: Parse and query EXPRESS schema metadata; it is metadata, not serialization and not the entity graph.

Follow `../AGENTS.md`. Read `PLAN.md` only when assigned implementation or
roadmap work; record progress and blockers there, not here.

## Boundary

Allowed production dependencies: `openbim-step` for its generic EXPRESS
syntax/model only; no production IFC crate. IFC version/profile integration
happens here and in consumers.

## Module ownership

- `express.rs`: thin adapter/re-export over `openbim_step::express`
- `model.rs`: declarations, inheritance, attributes, types
- `error.rs`: syntax/source diagnostics

## Invariants

- Official EXPRESS files are input evidence, never runtime/build dependencies.
- Preserve source names and version identity; do not guess cross-version equivalence.
- Schema queries are deterministic and do not interpret instance values.

Keep `lib.rs` delegating, keep child modules crate-private until they own a real
public contract, and split view/data, traversal, mutation, and validation before
they grow together.

## Verification

Run targeted crate tests and clippy first, then the package architecture/context
gates from `../AGENTS.md`. Record exact exit evidence in `PLAN.md`.
