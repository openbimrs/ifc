# ifc-validate where_rule instructions

Scope: explicit registry and bounded execution of supported WHERE rules. Follow the crate `../../AGENTS.md`. Read `PLAN.md` only for
assigned task(s) `VAL-WHERE` and keep implementation state there.

## Owns

- rule support registry
- rule input/output contract
- unsupported/failed/passed distinction
- execution budgets and diagnostics

## Does not own

- claiming all EXPRESS is executable
- kernel numerical algorithms
- silent skipped rules

## Growth map

`registry.rs`, `engine.rs`, `budget.rs`, `builtin.rs`. These source owners already compile as private scaffold modules. Replace a module's planned-owner marker with its first real contract and tests; do not add parallel placeholders. Every graph operation has deterministic order and explicit limits.
