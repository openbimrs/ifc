# ifc-xml instructions

Purpose: ifcXML codec adapter between XML and ifc-model.

Follow `../AGENTS.md`. Read `PLAN.md` only when assigned implementation or
roadmap work; record progress and blockers there, not here.

## Boundary

Allowed production dependencies: ifc-model; ifc-schema is optional naming metadata; ifc-step is test-only differential evidence.

## Module ownership

- `reader.rs`: namespace-aware XML to Model
- `writer.rs`: deterministic Model to XML
- `value.rs`: XML scalar/aggregate representation
- `error.rs`: XML and model conversion failures

## Invariants

- No domain semantics in the codec.
- Schema-disabled mode must remain useful and must not fabricate schema names.
- Unknown data survives semantically even when XML lexical form normalizes.

Keep `lib.rs` delegating, keep child modules crate-private until they own a real
public contract, and split view/data, traversal, mutation, and validation before
they grow together.

## Verification

Run targeted crate tests and clippy first, then the package architecture/context
gates from `../AGENTS.md`. Record exact exit evidence in `PLAN.md`.
