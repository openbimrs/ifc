# ifc-material layer instructions

Scope: material-layer identity and composition. Follow the crate `../../AGENTS.md`.
Read `PLAN.md` only for assigned task(s) `MAT-LAYER`; keep implementation state
there.

## Owns

- layer identity, material link, name, description, category, and priority
- authored layer thickness and ordered layer-set membership
- semantic association from a usage to its layer set

## Does not own

- layer-set usage direction, direction sense, offset, or reference extent
- offset transforms, wall solid generation, or quantity computation

Those geometry-affecting usage slots have one projection owner:
`ifc-geometry::input::material_usage`.

## Growth map

`definition.rs`, `set.rs`, `usage.rs`. These source owners already compile as
private scaffold modules. Replace a module's planned-owner marker with its first
real contract and tests; do not add parallel placeholders.
