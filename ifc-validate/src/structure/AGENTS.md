# ifc-validate structure instructions

Scope: schema structural checks over generic model records. Follow the crate `../../AGENTS.md`. Read `PLAN.md` only for
assigned task(s) `VAL-STRUCT` and keep implementation state there.

## Owns

- dangling/wrong-kind references
- required/optional/derived slot state
- aggregate cardinality and uniqueness

## Does not own

- codec parsing
- domain meaning
- WHERE-rule evaluation

## Growth map

`reference.rs`, `cardinality.rs`, `required.rs`, `unique.rs`. These source owners already compile as private scaffold modules. Replace a module's planned-owner marker with its first real contract and tests; do not add parallel placeholders. Every graph operation has deterministic order and explicit limits.
