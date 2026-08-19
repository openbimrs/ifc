# ifc-material constituent instructions

Scope: material constituent definitions and sets. Follow the crate `../../AGENTS.md`. Read `PLAN.md` only for
assigned task(s) `MAT-CONST` and keep implementation state there.

## Owns

- constituent identity/category/fraction/material link
- constituent set ordering/membership

## Does not own

- layer/profile geometry
- mixture simulation
- automatic fraction normalization

## Growth map

`definition.rs`, `set.rs`. These source owners already compile as private scaffold modules. Replace a module's planned-owner marker with its first real contract and tests; do not add parallel placeholders. Views borrow `ifc-model`; mutation waits
for an explicit model transaction contract.
