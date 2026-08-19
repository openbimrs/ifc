# ifc-geometry solid plan

Status: active scaffold under parent task(s) `GEOM-BREP, GEOM-SOLID`.
Last updated: 2026-08-19

Follow `AGENTS.md`. Claim one local task, leave blockers/decisions beneath it,
and check it off only after the proof runs.

## Work queue

- [ ] `SOLID-SLOTS` - verify all inherited absolute slots
  - Proof: focused tests, crate clippy, and relevant declaration/corpus gate.
- [ ] `SOLID-SWEPT` - complete swept-area/disk/fixed-reference views
  - Proof: focused tests, crate clippy, and relevant declaration/corpus gate.
- [ ] `SOLID-BOOL` - operand/operator/half-space semantics
  - Proof: focused tests, crate clippy, and relevant declaration/corpus gate.
- [ ] `SOLID-BREP` - shells/faces/topology references
  - Proof: focused tests, crate clippy, and relevant declaration/corpus gate.
- [ ] `SOLID-TESS` - coordinates/faces/normals/closed flags
  - Proof: focused tests, crate clippy, and relevant declaration/corpus gate.
- [ ] `SOLID-MODEL` - surface models and bounding boxes
  - Proof: focused tests, crate clippy, and relevant declaration/corpus gate.

## Completion log

Append `TASK-ID - proof - material decision`; keep long logs out of this file.
