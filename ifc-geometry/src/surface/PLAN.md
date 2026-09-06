# ifc-geometry surface plan

Status: active scaffold under parent task(s) `GEOM-SURFACE`.
Last updated: 2026-08-19

Follow `AGENTS.md`. Claim one local task, leave blockers/decisions beneath it,
and check it off only after the proof runs.

## Work queue

- [x] `SURF-SLOTS` - verify every surface accessor
  - Audited (2026-09-05): Every surface accessor is verified against generated absolute slots.
  - Proof: focused tests, crate clippy, and relevant declaration/corpus gate.
- [x] `SURF-ELEM` - frames/radii and degeneracy rules
  - Done (2026-09-05): degeneracy/radii logic was already complete; the four
    placement TODOs are now real typed accessors. `position(&model)` resolves
    an `Axis2Placement3D` view for plane, cylinder, sphere and torus.
  - Proof: focused tests, crate clippy, and relevant declaration/corpus gate.
- [x] `SURF-SWEPT` - extrusion/revolution surface inputs
  - Audited (2026-09-05): Extrusion and revolution surface inputs are read and tested.
  - Proof: focused tests, crate clippy, and relevant declaration/corpus gate.
- [x] `SURF-BOUND` - curve-bounded/rectangular trimmed semantics
  - Audited (2026-09-05): Curve-bounded and rectangular-trimmed semantics are covered, including sense flags.
  - Proof: focused tests, crate clippy, and relevant declaration/corpus gate.
- [x] `SURF-BSPLINE` - control grid/knots/weights validation
  - Audited (2026-09-05): Control grid, knots and weights are validated with typed errors.
  - Proof: focused tests, crate clippy, and relevant declaration/corpus gate.

## Completion log

Append `TASK-ID - proof - material decision`; keep long logs out of this file.
