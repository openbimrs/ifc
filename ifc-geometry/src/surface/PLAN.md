# ifc-geometry surface plan

Status: active scaffold under parent task(s) `GEOM-SURFACE`.
Last updated: 2026-08-19

Follow `AGENTS.md`. Claim one local task, leave blockers/decisions beneath it,
and check it off only after the proof runs.

## Work queue

- [x] `SURF-SLOTS` - verify every surface accessor
  - Audited (2026-09-05): Every surface accessor is verified against generated absolute slots.
  - Proof: focused tests, crate clippy, and relevant declaration/corpus gate.
- [ ] `SURF-ELEM` - frames/radii and degeneracy rules
  - Audited (2026-09-05): radii and degeneracy rules are complete and tested.
    Open remainder is narrow: four sites in `surface/elementary.rs` still
    return a raw `EntityId` for Position behind a `TODO: resource::placement
    will provide the typed placement view` marker. Lowering is unaffected -
    `lower/surface.rs` resolves those placements itself - so this is a typed-
    view refactor, not a missing capability.
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
