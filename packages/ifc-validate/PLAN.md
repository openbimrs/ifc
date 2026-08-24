# ifc-validate implementation plan

Status: architecture scaffold; validation now depends only on `ifc-model` and `ifc-schema`.
Last updated: 2026-08-19

This is task state, not ambient context. Follow `AGENTS.md`. Claim one task ID,
record blockers here, and check it off only with the stated evidence.

## Established boundary

Validate a Model against schema structure and registered semantic rules; never parse files itself.

## Planned file map

These paths are compiled private scaffold modules. Implement inside the named
owner and expose a public symbol only through an intentional parent re-export.

- `src/structure/reference.rs`: dangling/wrong-kind references
- `src/structure/cardinality.rs`: aggregate and optionality checks
- `src/structure/required.rs`: required attribute presence
- `src/structure/unique.rs`: UNIQUE rules and duplicate GUID reporting
- `src/type_check/entity.rs`: entity/subtype compatibility
- `src/type_check/select.rs`: SELECT membership
- `src/type_check/defined.rs`: defined/enumeration validation
- `src/type_check/enumeration.rs`: enumeration membership
- `src/type_check/scalar.rs`: scalar value form validation
- `src/where_rule/engine.rs`: bounded rule evaluation
- `src/where_rule/registry.rs`: explicit supported-rule registry
- `src/where_rule/builtin.rs`: audited native rule implementations
- `src/where_rule/budget.rs`: bounded evaluation and unsupported-rule limits
- `src/report/finding.rs`: structured diagnostics
- `src/report/summary.rs`: deterministic counts
- `src/report/path.rs`: entity/attribute source paths

## Work queue

- [x] `VAL-DEPS` - remove production dependency on ifc-step
  - Evidence: targeted tests plus crate clippy; add a focused fixture/property test.
- [ ] `VAL-STRUCT` - implement reference/cardinality checks from schema metadata
  - Evidence: targeted tests plus crate clippy; add a focused fixture/property test.
- [ ] `VAL-TYPE` - implement entity/select/defined-type compatibility
  - Evidence: targeted tests plus crate clippy; add a focused fixture/property test.
- [ ] `VAL-WHERE` - register supported rules and report unsupported ones honestly
  - Evidence: targeted tests plus crate clippy; add a focused fixture/property test.
- [ ] `VAL-REPORT` - deterministic reports with source paths and limits
  - Evidence: targeted tests plus crate clippy; add a focused fixture/property test.

## Completion log

Append concise entries as `TASK-ID - proof command/result - material decision`.
Do not paste long logs or transient process state.

- `VAL-DEPS` - removed `ifc-step`; focused crate check and package architecture
  gate pass, and a deliberate reintroduction is caught.
