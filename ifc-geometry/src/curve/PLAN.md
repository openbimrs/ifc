# ifc-geometry curve plan

Status: active scaffold under parent task(s) `GEOM-CURVE`.
Last updated: 2026-08-19

Follow `AGENTS.md`. Claim one local task, leave blockers/decisions beneath it,
and check it off only after the proof runs.

## Work queue

- [x] `CURVE-SLOTS` - verify inherited slots for every curve subtype
  - Audited (2026-09-05): Inherited slots verified per curve subtype with focused tests.
  - Proof: focused tests, crate clippy, and relevant declaration/corpus gate.
- [x] `CURVE-TRIM` - cover point/parameter trims, preference, sense, closed curves
  - Audited (2026-09-05): Point/parameter trims, preference, sense and closed curves are covered by trimmed.rs tests.
  - Proof: focused tests, crate clippy, and relevant declaration/corpus gate.
- [x] `CURVE-COMP` - continuity/transition and same-sense semantics
  - Audited (2026-09-05): Continuity/transition and same-sense semantics are read and tested in composite.rs.
  - Proof: focused tests, crate clippy, and relevant declaration/corpus gate.
- [x] `CURVE-BSPLINE` - knots/weights/multiplicity/degree validation
  - Audited (2026-09-05): Knots, weights, multiplicity and degree are validated with typed errors in bspline.rs.
  - Proof: focused tests, crate clippy, and relevant declaration/corpus gate.
- [x] `CURVE-LOWER-SEAM` - expose complete views needed by lower/curve.rs
  - Audited (2026-09-05): lower/curve.rs consumes the curve views directly; no competing reader remains.
  - Proof: focused tests, crate clippy, and relevant declaration/corpus gate.

## Completion log

Append `TASK-ID - proof - material decision`; keep long logs out of this file.
