# ifc-properties instructions

Purpose: Borrowed property, quantity, unit, template, and standard-library projections plus model authoring ports.

Follow `../AGENTS.md`. Read `PLAN.md` only for assigned implementation or
roadmap work; keep progress, blockers, and evidence there.

## Boundary

Allowed production dependencies: ifc-model and schema metadata; no geometry crate or backend.

## Module ownership

- `pset.rs`: property sets and property value forms
- `quantity.rs`: authored physical quantities and element quantity sets
- `unit.rs`: SI, conversion-based, derived, monetary, and unit assignment
- `template.rs`: property/quantity templates
- `standard.rs`: external property-set dictionaries
- `query.rs`: assignments and bounded lookup
- `value.rs`: semantic conversion from generic Value

## Invariants

- An IFC quantity is an authored assertion. This crate reads/writes it; it never computes shape measurements.
- Applications compute via a geometry service and pass the resulting typed value into authoring APIs.
- Units stay explicit; no bare f64 crosses a public quantity authoring boundary.

Keep cross-resource projections attribute-scoped: shared `ifc-model` storage
does not make one feature crate the owner of an IFC entity. Split typed views,
resolution, lowering, mutation, and validation before they grow together.

## Verification

Run targeted tests/clippy, isolated build, and the package architecture/context
gates. Geometry bridges also run declaration/corpus coverage and the full gate.
