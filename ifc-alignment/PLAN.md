# ifc-alignment implementation plan

Status: architecture scaffold; schema-version profile and segment lowering remain to implement.
Last updated: 2026-08-19

This is task state, not ambient context. Follow `AGENTS.md`; claim one task ID,
record blockers/decisions under it, and check it off only with evidence.

## Established boundary

Interpret IFC4x3 alignment intent into exact neutral curves/frames without meshing or backend selection.

## Planned file map

These paths are compiled private scaffold modules. Implement inside the named
owner and expose a public symbol only through an intentional parent re-export.

- `src/alignment/root.rs`: IfcAlignment hierarchy
- `src/horizontal/layout.rs`: segment order and continuity
- `src/horizontal/segment.rs`: line/arc/transition parameters
- `src/vertical/layout.rs`: profile order
- `src/vertical/segment.rs`: gradients/arcs/parabolas
- `src/cant/layout.rs`: cant segment order
- `src/cant/segment.rs`: cant transitions
- `src/curve/assemble.rs`: exact neutral composite curve
- `src/placement/linear.rs`: linear placement
- `src/placement/distance.rs`: point-by-distance expressions
- `src/referent/station.rs`: station referents

- `src/cant/transition.rs`: compiled private scaffold; implementation owned by `src/cant/PLAN.md`
- `src/curve/provenance.rs`: compiled private scaffold; implementation owned by `src/curve/PLAN.md`
- `src/curve/transition.rs`: compiled private scaffold; implementation owned by `src/curve/PLAN.md`
- `src/horizontal/transition.rs`: compiled private scaffold; implementation owned by `src/horizontal/PLAN.md`
- `src/placement/station.rs`: compiled private scaffold; implementation owned by `src/placement/PLAN.md`
- `src/vertical/transition.rs`: compiled private scaffold; implementation owned by `src/vertical/PLAN.md`

## Work queue

- [ ] `ALIGN-VERS` - pin the authoritative IFC4x3 profile and declaration inventory
  - Evidence: focused unit/property/fixture tests, isolated build, and crate clippy.
- [ ] `ALIGN-H` - implement exact horizontal segment views/lowering
  - Evidence: focused unit/property/fixture tests, isolated build, and crate clippy.
- [ ] `ALIGN-V` - implement exact vertical profile views/lowering
  - Evidence: focused unit/property/fixture tests, isolated build, and crate clippy.
- [ ] `ALIGN-CANT` - implement exact cant views/lowering
  - Evidence: focused unit/property/fixture tests, isolated build, and crate clippy.
- [ ] `ALIGN-CURVE` - assemble continuity-aware neutral curves
  - Evidence: focused unit/property/fixture tests, isolated build, and crate clippy.
- [ ] `ALIGN-PLACE` - implement linear placement and station equations
  - Evidence: focused unit/property/fixture tests, isolated build, and crate clippy.
- [ ] `ALIGN-CENSUS` - fixture/declaration coverage with explicit unsupported cases
  - Evidence: focused unit/property/fixture tests, isolated build, and crate clippy.

## Completion log

Append concise entries as `TASK-ID - proof command/result - material decision`.
Do not paste long logs or move standing invariants out of `AGENTS.md`.
