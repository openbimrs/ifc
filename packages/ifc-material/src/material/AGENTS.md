# ifc-material material instructions

Scope: material identity, category, and attached semantic properties. Follow the crate `../../AGENTS.md`. Read `PLAN.md` only for
assigned task(s) `MAT-BASE` and keep implementation state there.

## Owns

- IfcMaterial identity and category
- material property/representation associations as EntityId links

## Does not own

- surface rendering styles
- profile/layer geometry
- external material-library I/O

## Growth map

`definition.rs`, `properties.rs`, `relationships.rs` are the implementation owners. Extend them with focused tests; do not add parallel modules. Views borrow `ifc-model`; mutation waits for an explicit model transaction contract.
