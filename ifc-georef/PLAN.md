# ifc-georef implementation plan

Status: IFC4/IFC4X3 project-to-map conversion, projected-CRS/map-unit resolution, project-frame composition, true/grid/project north, and schema-pinned version dispatch implemented. Rigid-operation and scaled-factor coordinate operations remain a typed refusal (not lowered).
Last updated: 2026-09-06

This is task state, not ambient context. Follow `AGENTS.md`; claim one task ID,
record blockers/decisions under it, and check it off only with evidence.

## Established boundary

Interpret project-to-map/CRS operations and geodetic metadata; never place individual products.

## Planned file map

These paths are compiled private scaffold modules. Implement inside the named
owner and expose a public symbol only through an intentional parent re-export.

- `src/crs/projected.rs`: IfcProjectedCRS
- `src/crs/identifier.rs`: authority/name/datum metadata
- `src/conversion/map.rs`: IfcMapConversion parameters
- `src/conversion/rigid.rs`: rigid coordinate operations where schema permits
- `src/context/source.rs`: source context association
- `src/context/chain.rs`: project-frame to map-frame composition
- `src/north/directions.rs`: true/grid/project north
- `src/elevation/site.rs`: site/ref elevation semantics
- `src/view.rs`: IFC4/IFC4X3 schema pinning and version-aware entity lookup

- `src/conversion/validation.rs`: compiled private scaffold; implementation owned by `src/conversion/PLAN.md`
- `src/crs/unit.rs`: compiled private scaffold; implementation owned by `src/crs/PLAN.md`

## Work queue

- [x] `GEOREF-CRS` - implement CRS and map-unit views
  - Evidence: focused unit/property/fixture tests, isolated build, and crate clippy.
- [x] `GEOREF-MAP` - implement map-conversion transform with degenerate-axis checks
  - Evidence: focused unit/property/fixture tests, isolated build, and crate clippy.
- [x] `GEOREF-CHAIN` - define/test composition with a separately supplied project frame
  - Evidence: `context/chain.rs::compose_project_frame`, tested for translation, singular-frame refusal, and non-identity composition order; `cargo +1.88.0 test -p ifc-georef`.
- [x] `GEOREF-NORTH` - distinguish and test true, grid, and project north
  - Evidence: `north/directions.rs` -- `NorthReference::{Project,True,Grid}`, `resolve_true_north` (IFC's documented `(0,1)` default), `grid_north_direction` (derived from the map conversion's own rotation); integration test proves all three can disagree simultaneously.
- [x] `GEOREF-VERS` - specify IFC4 versus IFC4x3 coordinate-operation profiles
  - Evidence: `view.rs::GeorefView` pins IFC4/IFC4X3 (refusing IFC2X3, which declares no georeferencing entities at all -- verified against `IFC2X3_TC1.exp`), `resolve_project_to_map_in` distinguishes an IFC4X3-only entity read under IFC4 from a wholly wrong entity; attribute-slot identity between IFC4/IFC4X3 confirmed against both `.exp` files (see `view.rs` module doc).
- [x] `GEOREF-CORPUS` - validate against independently known coordinate examples
  - Evidence: focused unit/property/fixture tests, isolated build, and crate clippy.

## Completion log

Append concise entries as `TASK-ID - proof command/result - material decision`.

- `GEOREF-CRS/MAP/CORPUS` - `cargo +1.88.0 test -p ifc-georef` plus the
  workspace gate pass; source and map units normalize to metres, axes normalize
  or fail closed, and a committed IFC fixture proves the neutral transform.
- `GEOREF-CHAIN/NORTH/VERS` - `cargo +1.88.0 test -p ifc-georef` (30 tests,
  up from 8) plus the workspace gate pass. Project-frame composition refuses
  singular frames rather than propagating NaN; true/grid/project north are
  kept distinguishable rather than collapsed, matching IFC's own stated
  precedence (map conversion authoritative over true north when both are
  present); IFC4/IFC4X3 dispatch pins the schema and names an IFC4X3-only
  entity read under IFC4 as a schema mismatch rather than a generic
  wrong-type error. IFC2X3 remains refused (no georeferencing entities
  exist in that schema at all).

Do not paste long logs or move standing invariants out of `AGENTS.md`.
