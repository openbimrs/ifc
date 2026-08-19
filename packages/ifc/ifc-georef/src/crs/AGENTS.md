# ifc-georef crs instructions

Scope: coordinate reference system identity, datum, axis, and map-unit metadata. Follow the crate `../../AGENTS.md`. Read `PLAN.md` only for
assigned task(s) `GEOREF-CRS`; keep progress and blockers there.

## Owns

- IfcCoordinateReferenceSystem/ProjectedCRS views
- authority/name/datum/map unit

## Does not own

- network CRS lookup
- coordinate reprojection library bindings
- product placements

## Growth map

`projected.rs`, `identifier.rs`, `unit.rs`. These source owners already compile as private scaffold modules. Replace a module's planned-owner marker with its first real contract and tests; do not add parallel placeholders. Keep views, resolution, validation, and neutral output in
separate files.
