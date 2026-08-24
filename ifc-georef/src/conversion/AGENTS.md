# ifc-georef conversion instructions

Scope: project-context to map-coordinate operations. Follow the crate `../../AGENTS.md`. Read `PLAN.md` only for
assigned task(s) `GEOREF-MAP`; keep progress and blockers there.

## Owns

- IfcMapConversion parameters
- scale/axis/translation validation
- neutral transform output

## Does not own

- mutating geometry
- backend execution
- local product placement chains

## Growth map

`map.rs`, `rigid.rs`, `validation.rs`. These source owners already compile as private scaffold modules. Replace a module's planned-owner marker with its first real contract and tests; do not add parallel placeholders. Keep views, resolution, validation, and neutral output in
separate files.
