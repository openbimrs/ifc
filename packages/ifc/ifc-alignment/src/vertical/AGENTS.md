# ifc-alignment vertical instructions

Scope: vertical profile and exact gradient segment parameters. Follow the crate `../../AGENTS.md`. Read `PLAN.md` only for
assigned task(s) `ALIGN-V`; keep progress and blockers there.

## Owns

- vertical segment order
- constant-gradient/parabolic/circular profile inputs
- station/elevation continuity

## Does not own

- 3D meshing
- cant combination
- earthwork computation

## Growth map

`layout.rs`, `segment.rs`, `transition.rs`. These source owners already compile as private scaffold modules. Replace a module's planned-owner marker with its first real contract and tests; do not add parallel placeholders. Keep views, resolution, validation, and neutral output in
separate files.
