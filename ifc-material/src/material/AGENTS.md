# ifc-material material instructions

Scope: material identity, category, and attached semantic properties. Follow the crate `../../AGENTS.md`.

## Owns

- IfcMaterial identity and category
- material property/representation associations as EntityId links

## Does not own

- surface rendering styles
- profile/layer geometry
- external material-library I/O

## Growth map

`definition.rs`, `properties.rs`, `relationships.rs` are the implementation owners. Extend them with focused tests; do not add parallel modules. Views borrow `ifc-model`; writes stage through `src/authoring` on a caller-owned `ifc-model::Transaction`.
