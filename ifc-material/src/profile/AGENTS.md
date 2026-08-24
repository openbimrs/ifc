# ifc-material profile instructions

Scope: semantic attributes of `IfcMaterialProfile*`. Follow the crate
`../../AGENTS.md`. Read `PLAN.md` only for assigned task(s) `MAT-PROFILE`; keep
implementation state there.

## Owns

- material/profile links, name, description, priority, and category
- profile-set identity, description, ordered membership, and composite indicator
- authored usage cardinal points, extents, offsets, and taper end-set links

## Does not own

- profile shape evaluation and cardinal placement math
- start/end taper interpolation or sweep construction

`ifc-geometry::input::material_usage` may project the same raw IFC record to
remain domain-independent, but it owns lowering rather than material semantics.

## Growth map

`definition.rs`, `set.rs`, and `usage.rs` are the implementation owners. Extend
them with focused tests; do not add parallel modules.
