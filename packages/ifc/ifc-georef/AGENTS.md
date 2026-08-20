# ifc-georef instructions

Purpose: Interpret project-to-map/CRS operations and geodetic metadata; never place individual products.

Follow `../AGENTS.md`. Read `PLAN.md` only for assigned implementation or
roadmap work; keep progress, blockers, and evidence there.

## Boundary

Allowed production dependencies: ifc-model, schema metadata, and neutral axiolid-core transform/value types only.

## Module ownership

- `crs.rs`: projected CRS identity, datum, map unit
- `conversion.rs`: IfcMapConversion and coordinate operations
- `context.rs`: source representation context linkage
- `elevation.rs`: site/ref elevation metadata
- `north.rs`: true/project/grid north distinctions
- `error.rs`: incomplete/invalid CRS operations

## Invariants

- Ifc local/product placement is ifc-geometry; project-to-world/map conversion is this crate.
- Output is a neutral transform plus CRS metadata, not reprojected geometry and not a GIS side effect.
- Map units and axis order are explicit; no implicit metre/easting/northing assumptions.

Keep cross-resource projections attribute-scoped: shared `ifc-model` storage
does not make one feature crate the owner of an IFC entity. Split typed views,
resolution, lowering, mutation, and validation before they grow together.

## Verification

Run targeted tests/clippy, isolated build, and the package architecture/context
gates. Geometry bridges also run declaration/corpus coverage and the full gate.
