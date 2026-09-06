# ifc-geometry select plan

Status: active scaffold under parent task(s) `GEOM-CENSUS`.
Last updated: 2026-08-19

Follow `AGENTS.md`. Claim one local task, leave blockers/decisions beneath it,
and check it off only after the proof runs.

## Work queue

- [x] `SEL-MANIFEST` - regenerate/check subtype table against authoritative schema
  - Audited (2026-09-05): tests/schema_coverage.rs checks the subtype table against the generated inventory and fails on drift.
  - Proof: focused tests, crate clippy, and relevant declaration/corpus gate.
- [x] `SEL-POSNEG` - pair every family with valid and invalid membership tests
  - Done (2026-09-05): the four families that resolved without focused
    membership tests now have them: `CsgSelect`, `CurveOrEdgeCurve`,
    `SurfaceOrFaceSurface`, `PointOrVertexPoint`. Each pairs the valid
    branches with a rejected non-member, and the three subtype-ordering
    cases assert the most-derived branch wins.
  - Proof: swapping the `PointOrVertexPoint` branch order fails
    `a_vertex_point_keeps_its_topological_identity`.
- [x] `SEL-VERS` - make schema-version identity explicit before IFC4x3 support
  - Done (2026-09-05): `subtype::TABLE_SCHEMA_VERSION` names the schema the
    compiled chains encode, and `tables_are_verified_for` lets a caller ask
    before trusting them for another version. Previously the version was
    prose only, so nothing failed if a caller assumed IFC4X3.
  - Proof: `the_declared_table_version_resolves_the_compiled_chains` walks
    every compiled chain against the named bundled schema; pointing the
    constant at IFC2X3 fails on `IFCADVANCEDBREP`.

## Completion log

Append `TASK-ID - proof - material decision`; keep long logs out of this file.
