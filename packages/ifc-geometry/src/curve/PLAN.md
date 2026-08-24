# ifc-geometry curve plan

Status: active scaffold under parent task(s) `GEOM-CURVE`.
Last updated: 2026-08-19

Follow `AGENTS.md`. Claim one local task, leave blockers/decisions beneath it,
and check it off only after the proof runs.

## Work queue

- [ ] `CURVE-SLOTS` - verify inherited slots for every curve subtype
  - Proof: focused tests, crate clippy, and relevant declaration/corpus gate.
- [ ] `CURVE-TRIM` - cover point/parameter trims, preference, sense, closed curves
  - Proof: focused tests, crate clippy, and relevant declaration/corpus gate.
- [ ] `CURVE-COMP` - continuity/transition and same-sense semantics
  - Proof: focused tests, crate clippy, and relevant declaration/corpus gate.
- [ ] `CURVE-BSPLINE` - knots/weights/multiplicity/degree validation
  - Proof: focused tests, crate clippy, and relevant declaration/corpus gate.
- [ ] `CURVE-LOWER-SEAM` - expose complete views needed by lower/curve.rs
  - Proof: focused tests, crate clippy, and relevant declaration/corpus gate.

## Completion log

Append `TASK-ID - proof - material decision`; keep long logs out of this file.
