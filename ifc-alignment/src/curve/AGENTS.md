# ifc-alignment curve instructions

Scope: assemble exact neutral alignment curves from three design axes. Follow the crate `../../AGENTS.md`. Read `PLAN.md` only for
assigned task(s) `ALIGN-CURVE`; keep progress and blockers there.

## Owns

- neutral composite curve construction
- segment transition mapping
- provenance from neutral segment to IFC segment

## Does not own

- tessellation
- kernel/backend calls
- schema view definitions

## Growth map

`assemble.rs`, `transition.rs`, `provenance.rs`. These source owners already compile as private scaffold modules. Replace a module's planned-owner marker with its first real contract and tests; do not add parallel placeholders. Keep views, resolution, validation, and neutral output in
separate files.
