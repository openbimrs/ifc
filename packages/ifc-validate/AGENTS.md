# ifc-validate instructions

Purpose: Validate a Model against schema structure and registered semantic rules; never parse files itself.

Follow `../AGENTS.md`. Read `PLAN.md` only when assigned implementation or
roadmap work; record progress and blockers there, not here.

## Boundary

Allowed production dependencies: ifc-model and ifc-schema only.

## Module ownership

- `structure.rs`: references, cardinality, required slots
- `type_check.rs`: entity/select/defined/enumeration compatibility
- `where_rule.rs`: bounded rule registry/execution
- `report.rs`: stable findings and summaries

## Invariants

- Validation consumes a Model; codec adapters stay outside.
- Findings cite entity, attribute/rule, severity, and evidence path.
- Unsupported rules are reported separately from passing rules.

Keep `lib.rs` delegating, keep child modules crate-private until they own a real
public contract, and split view/data, traversal, mutation, and validation before
they grow together.

## Verification

Run targeted crate tests and clippy first, then the package architecture/context
gates from `../AGENTS.md`. Record exact exit evidence in `PLAN.md`.
