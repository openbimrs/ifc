# ifc-material instructions

Purpose: Borrowed semantic projections for materials, layers, profiles, constituents, and their usage/assignment.

Follow `../AGENTS.md`. Read `PLAN.md` only for assigned implementation or
roadmap work; keep progress, blockers, and evidence there.

## Boundary

Allowed production dependencies: ifc-model and optional schema metadata; no geometry crate.

## Module ownership

- `material.rs`: material identity/category/properties
- `layer.rs`: layers, sets, and usage semantics
- `profile.rs`: material-profile identity, set membership, priority/category
- `constituent.rs`: constituent sets and fractions
- `usage.rs`: product associations and semantic resolution
- `error.rs`: malformed/ambiguous material projections

## Invariants

- Profile shape math, cardinal placement, layer offsets, and taper geometry are read independently by ifc-geometry.
- This crate may expose EntityId references to profiles but never constructs geom profiles or transforms.
- Resolve assignments with explicit ambiguity/cycle behavior; do not guess a winning material association.

Keep cross-resource projections attribute-scoped: shared `ifc-model` storage
does not make one feature crate the owner of an IFC entity. Split typed views,
resolution, lowering, mutation, and validation before they grow together.

## Verification

Run targeted tests/clippy, isolated build, and the package architecture/context
gates. Geometry bridges also run declaration/corpus coverage and the full gate.
