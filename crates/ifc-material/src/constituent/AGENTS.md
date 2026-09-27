# ifc-material constituent instructions

Scope: material constituent definitions and sets. Follow the crate `../../AGENTS.md`.

## Owns

- constituent identity/category/fraction/material link
- constituent set membership; source order is preserved only as a deterministic projection of the normative SET, not as semantic order

## Does not own

- layer/profile geometry
- mixture simulation
- automatic fraction normalization. IFC4 has no WHERE rule requiring a set's
  fractions to sum to 1, so a sum check is an opt-in policy diagnostic (#103),
  never a decode or schema error

## Growth map

`definition.rs` and `set.rs` are the implementation owners. Extend them with focused tests; do not add parallel modules. Views borrow `ifc-model`; writes stage through `src/authoring` on a caller-owned `ifc-model::Transaction`.
