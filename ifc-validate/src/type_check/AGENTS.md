# ifc-validate type_check instructions

Scope: schema type compatibility for entity attributes and values. Follow the crate `../../AGENTS.md`.

## Owns

- entity subtype compatibility
- SELECT membership
- defined/enumeration/logical/scalar forms

## Does not own

- geometry/domain validation
- implicit numeric coercion
- runtime code generation

## Growth map

`entity.rs`, `select.rs`, `defined.rs`, `enumeration.rs`, `scalar.rs`, `aggregate.rs`. These source owners already compile as private scaffold modules. Replace a module's planned-owner marker with its first real contract and tests; do not add parallel placeholders. Every graph operation has deterministic order and explicit limits.
