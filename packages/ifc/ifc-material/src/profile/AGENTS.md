# ifc-material profile instructions

Scope: semantic attributes of `IfcMaterialProfile*`. Follow the crate
`../../AGENTS.md`. Read `PLAN.md` only for assigned task(s) `MAT-PROFILE`; keep
implementation state there.

## Owns

- material link, name, description, priority, and category
- profile-set identity, description, ordered membership, and composite indicator
- semantic association from a usage to its profile set

## Does not own

- profile geometry reference or shape evaluation
- cardinal point, reference extent, or authored offsets
- start/end taper geometry association or sweep construction

Those geometry-affecting slots have one projection owner:
`ifc-geometry::input::material_usage`.

## Growth map

`definition.rs`, `set.rs`, `usage.rs`. These source owners already compile as
private scaffold modules. Replace a module's planned-owner marker with its first
real contract and tests; do not add parallel placeholders.
