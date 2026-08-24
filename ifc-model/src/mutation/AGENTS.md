# ifc-model mutation instructions

Scope: schema-agnostic transactional edits to records and values. Follow the crate `../../AGENTS.md`. Read `PLAN.md` only for
assigned task(s) `MODEL-MUT` and keep implementation state there.

## Owns

- explicit edit operations
- preflight/conflict checks
- atomic commit and index updates

## Does not own

- material/property/domain setters
- partial mutation on failure
- automatic schema repair

## Growth map

`edit.rs`, `transaction.rs`, `conflict.rs`. These source owners already compile as private scaffold modules. Replace a module's planned-owner marker with its first real contract and tests; do not add parallel placeholders. Every graph operation has deterministic order and explicit limits.
