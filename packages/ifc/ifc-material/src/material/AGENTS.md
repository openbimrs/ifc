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

`definition.rs`, `properties.rs`, `relationships.rs`. These source owners already compile as private scaffold modules. Replace a module's planned-owner marker with its first real contract and tests; do not add parallel placeholders. Views borrow `ifc-model`; mutation waits
for an explicit model transaction contract.
