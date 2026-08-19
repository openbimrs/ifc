# ifc-material usage instructions

Scope: product/type material associations and deterministic semantic resolution. Follow the crate `AGENTS.md`. Read `PLAN.md` only for
assigned task(s) `MAT-ASSIGN` and keep implementation state there.

## Owns

- RelAssociatesMaterial projections
- occurrence/type association traversal
- ambiguity and cycle diagnostics

## Does not own

- choosing geometry placement from material usage
- mutating assignments without model transaction
- depending on product-domain crates

## Growth map

`assignment.rs`, `resolution.rs`. These source owners already compile as private scaffold modules. Replace a module's planned-owner marker with its first real contract and tests; do not add parallel placeholders. Views borrow `ifc-model`; mutation waits
for an explicit model transaction contract.
