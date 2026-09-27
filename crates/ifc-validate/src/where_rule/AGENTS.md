# ifc-validate where_rule instructions

Scope: explicit registry and bounded execution of supported WHERE rules. Follow the crate `../../AGENTS.md`.

## Owns

- rule support registry
- rule input/output contract
- unsupported/undecided/failed/passed distinction
- execution budgets and diagnostics

## Does not own

- claiming all EXPRESS is executable
- kernel numerical algorithms
- silent skipped rules: an implemented rule that applies to an instance but
  cannot read its operands reports `Severity::EvaluationError` through
  `operand.rs`, never a bare `continue`

## Growth map

`registry.rs`, `engine.rs`, `budget.rs`, `builtin.rs`, `operand.rs`. These source owners already compile as private scaffold modules. Replace a module's planned-owner marker with its first real contract and tests; do not add parallel placeholders. Every graph operation has deterministic order and explicit limits.
