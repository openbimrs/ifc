# ifc-properties unit instructions

Scope: dimensional project and per-value unit interpretation. Follow the crate `../../AGENTS.md`. Read `PLAN.md` only for
assigned task(s) `PROP-UNIT` and keep implementation state there.

## Owns

- SI prefix/dimension semantics
- conversion-based and derived units
- unit assignment and explicit overrides

## Does not own

- global mutable unit state
- geometry transforms
- guessing missing dimensions from property names

## Growth map

`assignment.rs`, `si.rs`, `conversion.rs`, `derived.rs`, `monetary.rs`. These source owners already compile as private scaffold modules. Replace a module's planned-owner marker with its first real contract and tests; do not add parallel placeholders. Views borrow `ifc-model`; mutation waits
for an explicit model transaction contract.
