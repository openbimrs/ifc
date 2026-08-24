# ifc-material layer instructions

Scope: material-layer identity and composition. Follow the crate `../../AGENTS.md`.
Read `PLAN.md` only for assigned task(s) `MAT-LAYER`; keep implementation state
there.

## Owns

- layer identity, material link, name, description, category, and priority
- authored layer thickness and ordered layer-set membership
- authored usage direction, sense, offset, extent, and offset-layer values

## Does not own

- geometric interpretation of directions or offsets
- offset transforms, wall solid generation, or quantity computation

`ifc-geometry::input::material_usage` may project the same raw IFC record to
remain domain-independent, but it owns lowering rather than material semantics.

## Growth map

`definition.rs`, `set.rs`, and `usage.rs` are the implementation owners. Extend
them with focused tests; do not add parallel modules.
