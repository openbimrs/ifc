# ifc-geometry solid plan

Status: active scaffold under parent task(s) `GEOM-BREP, GEOM-SOLID`.
Last updated: 2026-08-19

Follow `AGENTS.md`. Claim one local task, leave blockers/decisions beneath it,
and check it off only after the proof runs.

## Work queue

- [x] `SOLID-SLOTS` - verify all inherited absolute slots
  - Audited (2026-09-05): All inherited absolute slots verified per solid subtype.
  - Proof: focused tests, crate clippy, and relevant declaration/corpus gate.
- [x] `SOLID-SWEPT` - complete swept-area/disk/fixed-reference views
  - Audited (2026-09-05): Swept-area, swept-disk and fixed-reference views are complete and tested.
  - Proof: focused tests, crate clippy, and relevant declaration/corpus gate.
- [x] `SOLID-BOOL` - operand/operator/half-space semantics
  - Audited (2026-09-05): Operand, operator and half-space semantics are covered, including the polygonal bound.
  - Proof: focused tests, crate clippy, and relevant declaration/corpus gate.
- [x] `SOLID-BREP` - shells/faces/topology references
  - Audited (2026-09-05): Shell, face and topology references are read through resource::topology and solid::brep.
  - Proof: focused tests, crate clippy, and relevant declaration/corpus gate.
- [x] `SOLID-TESS` - coordinates/faces/normals/closed flags
  - Audited (2026-09-05): Coordinates, faces, normals and closed flags are decoded with typed errors.
  - Proof: focused tests, crate clippy, and relevant declaration/corpus gate.
- [x] `SOLID-MODEL` - surface models and bounding boxes
  - Audited (2026-09-05): Surface models and bounding boxes lower through the shared collection path.
  - Proof: focused tests, crate clippy, and relevant declaration/corpus gate.

## Completion log

Append `TASK-ID - proof - material decision`; keep long logs out of this file.
