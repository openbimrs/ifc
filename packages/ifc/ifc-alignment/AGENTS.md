# ifc-alignment instructions

Purpose: Interpret IFC4x3 alignment intent into exact neutral curves/frames without meshing or backend selection.

Follow `../AGENTS.md`. Read `PLAN.md` only for assigned implementation or
roadmap work; keep progress, blockers, and evidence there.

## Boundary

Allowed production dependencies: ifc-model, schema metadata, and exact neutral axiolid-core/axiolid-curve representations; axiolid-model only if graph output is required.

## Module ownership

- `alignment.rs`: root/nesting/representation association
- `horizontal.rs`: horizontal layout and segment parameters
- `vertical.rs`: vertical profile segments
- `cant.rs`: cant/superelevation segments
- `segment.rs`: shared segment transitions/continuity
- `curve.rs`: exact neutral curve assembly
- `placement.rs`: linear placement/point-by-distance
- `referent.rs`: stationing/referents
- `query.rs`: bounded alignment traversal

## Invariants

- Preserve exact transition intent; tessellation is a downstream explicit operation.
- Station, horizontal length, projected length, and 3D length are distinct quantities.
- No road/rail product workflow or rendering policy enters this bridge.

Keep cross-resource projections attribute-scoped: shared `ifc-model` storage
does not make one feature crate the owner of an IFC entity. Split typed views,
resolution, lowering, mutation, and validation before they grow together.

## Verification

Run targeted tests/clippy, isolated build, and the package architecture/context
gates. Geometry bridges also run declaration/corpus coverage and the full gate.
