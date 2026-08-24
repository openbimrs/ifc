# ifc-material constituent instructions

Scope: material constituent definitions and sets. Follow the crate `../../AGENTS.md`. Read `PLAN.md` only for
assigned task(s) `MAT-CONST` and keep implementation state there.

## Owns

- constituent identity/category/fraction/material link
- constituent set membership; source order is preserved only as a deterministic projection of the normative SET, not as semantic order

## Does not own

- layer/profile geometry
- mixture simulation
- automatic fraction normalization

## Growth map

`definition.rs` and `set.rs` are the implementation owners. Extend them with focused tests; do not add parallel modules. Views borrow `ifc-model`; mutation waits for an explicit model transaction contract.
