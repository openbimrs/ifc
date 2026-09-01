# ifc-geometry input plan

Status: active scaffold under parent task(s) `GEOM-INPUT`.
Last updated: 2026-09-01

Follow `AGENTS.md`. Claim one local task, leave blockers/decisions beneath it,
and check it off only after the proof runs.

## Work queue

- [ ] `INPUT-PROFILE` - exact profile/resource views with absolute-slot tests
  - Proof: focused tests, crate clippy, and relevant declaration/corpus gate.
- [x] `INPUT-REP` - context and representation-selection views
  - Proof: focused tests, crate clippy, and relevant declaration/corpus gate.
- [x] `INPUT-MAT` - geometry-only views for profile references, cardinal/reference extent, layer direction/sense/offset, taper associations, and profile offsets
  - Proof: four absolute-slot/error unit tests and clippy in both feature columns.
  - Decision: sibling crates cannot depend on each other, so `ifc-geometry` and
    `ifc-material` deliberately project their own attribute-scoped views from
    shared `ifc-model` records; material identity never enters Axiolid.
- [x] `INPUT-PRODUCT` - product shape and placement links
  - Proof: focused tests, crate clippy, and relevant declaration/corpus gate.
- [ ] `INPUT-TOPO` - topology views required by B-rep
  - Proof: focused tests, crate clippy, and relevant declaration/corpus gate.

## Completion log

`INPUT-MAT` - four focused tests and both feature-column clippy checks passed;
views remain geometry-only and values stay in project units.

Append `TASK-ID - proof - material decision`; keep long logs out of this file.
