# ifc-geometry input plan

Status: active scaffold under parent task(s) `GEOM-INPUT`.
Last updated: 2026-08-19

Follow `AGENTS.md`. Claim one local task, leave blockers/decisions beneath it,
and check it off only after the proof runs.

## Work queue

- [ ] `INPUT-PROFILE` - exact profile/resource views with absolute-slot tests
  - Proof: focused tests, crate clippy, and relevant declaration/corpus gate.
- [x] `INPUT-REP` - context and representation-selection views
  - Proof: focused tests, crate clippy, and relevant declaration/corpus gate.
- [ ] `INPUT-MAT` - single-owner views for profile references, cardinal/reference extent, layer usage direction/sense/offset, and taper geometry associations
  - Proof: absolute-slot tests plus a cross-projection fixture proving `ifc-material` does not parse these slots.
- [x] `INPUT-PRODUCT` - product shape and placement links
  - Proof: focused tests, crate clippy, and relevant declaration/corpus gate.
- [ ] `INPUT-TOPO` - topology views required by B-rep
  - Proof: focused tests, crate clippy, and relevant declaration/corpus gate.

## Completion log

Append `TASK-ID - proof - material decision`; keep long logs out of this file.
