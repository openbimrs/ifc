# ifc-material implementation plan

Status: architecture scaffold; all semantic material families remain to implement.
Last updated: 2026-08-19

This is task state, not ambient context. Follow `AGENTS.md`; claim one task ID,
record blockers/decisions under it, and check it off only with evidence.

## Established boundary

Borrowed semantic projections for materials, layers, profiles, constituents, and their usage/assignment.

## Planned file map

These paths are compiled private scaffold modules. Implement inside the named
owner and expose a public symbol only through an intentional parent re-export.

- `src/material/definition.rs`: IfcMaterial identity
- `src/material/properties.rs`: material property relationships
- `src/layer/definition.rs`: identity, material link, metadata, and authored thickness
- `src/layer/set.rs`: ordered layer sets
- `src/layer/usage.rs`: semantic association to a layer set only
- `src/profile/definition.rs`: material/name/description/priority/category
- `src/profile/set.rs`: ordered profile sets
- `src/profile/usage.rs`: semantic association to a profile set only
- `src/constituent/definition.rs`: constituent semantics
- `src/constituent/set.rs`: set membership
- `src/usage/assignment.rs`: RelAssociatesMaterial view
- `src/usage/resolution.rs`: bounded association resolution

- `src/material/relationships.rs`: compiled private scaffold; implementation owned by `src/material/PLAN.md`

## Work queue

- [ ] `MAT-BASE` - implement material identity and property relationships
  - Evidence: focused unit/property/fixture tests, isolated build, and crate clippy.
- [ ] `MAT-LAYER` - implement layer composition without geometry-usage slots
  - Evidence: focused unit/property/fixture tests, isolated build, and crate clippy.
- [ ] `MAT-PROFILE` - implement only the semantic attributes of IfcMaterialProfile*
  - Evidence: focused unit/property/fixture tests, isolated build, and crate clippy.
- [ ] `MAT-CONST` - implement constituents and fractions with validation
  - Evidence: focused unit/property/fixture tests, isolated build, and crate clippy.
- [ ] `MAT-ASSIGN` - resolve product/type material associations deterministically
  - Evidence: focused unit/property/fixture tests, isolated build, and crate clippy.
- [ ] `MAT-MUT` - add authoring only after MODEL-MUT exists
  - Evidence: focused unit/property/fixture tests, isolated build, and crate clippy.
- [ ] `MAT-CROSS` - prove material and geometry projections join by EntityId without duplicate slot ownership
  - Requires: `MAT-LAYER`, `MAT-PROFILE`, `INPUT-MAT`.
  - Evidence: cross-projection fixtures, isolated build, and crate clippy.

## Completion log

Append concise entries as `TASK-ID - proof command/result - material decision`.
Do not paste long logs or move standing invariants out of `AGENTS.md`.
