# ifc-georef implementation plan

Status: architecture scaffold; map conversion and CRS projections remain to implement.
Last updated: 2026-08-19

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
- `src/context/chain.rs`: project-frame to map-frame composition contract
- `src/north/directions.rs`: true/grid/project north
- `src/elevation/site.rs`: site/ref elevation semantics

- `src/conversion/validation.rs`: compiled private scaffold; implementation owned by `src/conversion/PLAN.md`
- `src/crs/unit.rs`: compiled private scaffold; implementation owned by `src/crs/PLAN.md`

## Work queue

- [ ] `GEOREF-CRS` - implement CRS and map-unit views
  - Evidence: focused unit/property/fixture tests, isolated build, and crate clippy.
- [ ] `GEOREF-MAP` - implement map-conversion transform with degenerate-axis checks
  - Evidence: focused unit/property/fixture tests, isolated build, and crate clippy.
- [ ] `GEOREF-CHAIN` - define/test composition with a separately supplied project frame
  - Evidence: focused unit/property/fixture tests, isolated build, and crate clippy.
- [ ] `GEOREF-NORTH` - distinguish and test true, grid, and project north
  - Evidence: focused unit/property/fixture tests, isolated build, and crate clippy.
- [ ] `GEOREF-VERS` - specify IFC4 versus IFC4x3 coordinate-operation profiles
  - Evidence: focused unit/property/fixture tests, isolated build, and crate clippy.
- [ ] `GEOREF-CORPUS` - validate against independently known coordinate examples
  - Evidence: focused unit/property/fixture tests, isolated build, and crate clippy.

## Completion log

Append concise entries as `TASK-ID - proof command/result - material decision`.
Do not paste long logs or move standing invariants out of `AGENTS.md`.
