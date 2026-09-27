# ifc-material usage instructions

Scope: product/type material associations and deterministic semantic resolution. Follow the crate `../../AGENTS.md`.

## Owns

- RelAssociatesMaterial projections
- occurrence/type association traversal
- ambiguity and cycle diagnostics

## Does not own

- choosing geometry placement from material usage
- mutating assignments without model transaction
- depending on product-domain crates

## Growth map

`assignment.rs` and `resolution.rs` are the implementation owners. Extend them with focused tests; do not add parallel modules. Views borrow `ifc-model`; writes stage through `src/authoring` on a caller-owned `ifc-model::Transaction`.
