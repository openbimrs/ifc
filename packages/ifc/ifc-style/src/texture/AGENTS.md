# ifc-style texture instructions

Scope: texture descriptors and coordinate associations. Follow the crate `../../AGENTS.md`. Read `PLAN.md` only for
assigned task(s) `STYLE-TEXTURE`; keep progress and blockers there.

## Owns

- texture metadata/repeat/mode
- image/blob references as data
- texture coordinate/map associations

## Does not own

- image decoding or I/O
- UV generation from geometry
- renderer handles

## Growth map

`surface.rs`, `image.rs`, `coordinate.rs`, `map.rs`. These source owners already compile as private scaffold modules. Replace a module's planned-owner marker with its first real contract and tests; do not add parallel placeholders. Keep views, resolution, validation, and neutral output in
separate files.
