# ifc-geometry surface plan

Status: active scaffold under parent task(s) `GEOM-SURFACE`.
Last updated: 2026-08-19

Follow `AGENTS.md`. Claim one local task, leave blockers/decisions beneath it,
and check it off only after the proof runs.

## Work queue

- [ ] `SURF-SLOTS` - verify every surface accessor
  - Proof: focused tests, crate clippy, and relevant declaration/corpus gate.
- [ ] `SURF-ELEM` - frames/radii and degeneracy rules
  - Proof: focused tests, crate clippy, and relevant declaration/corpus gate.
- [ ] `SURF-SWEPT` - extrusion/revolution surface inputs
  - Proof: focused tests, crate clippy, and relevant declaration/corpus gate.
- [ ] `SURF-BOUND` - curve-bounded/rectangular trimmed semantics
  - Proof: focused tests, crate clippy, and relevant declaration/corpus gate.
- [ ] `SURF-BSPLINE` - control grid/knots/weights validation
  - Proof: focused tests, crate clippy, and relevant declaration/corpus gate.

## Completion log

Append `TASK-ID - proof - material decision`; keep long logs out of this file.
