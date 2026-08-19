# ifc-model index instructions

Scope: derived lookup structures coherent with the generic entity graph. Follow the crate `../../AGENTS.md`. Read `PLAN.md` only for
assigned task(s) `MODEL-INV` and keep implementation state there.

## Owns

- type index
- reverse reference index
- optional secondary index lifecycle

## Does not own

- domain-specific indexes
- stale cache after mutation
- mandatory indexes without measured value

## Growth map

`type_index.rs`, `reverse.rs`, `builder.rs`. These source owners already compile as private scaffold modules. Replace a module's planned-owner marker with its first real contract and tests; do not add parallel placeholders. Every graph operation has deterministic order and explicit limits.
