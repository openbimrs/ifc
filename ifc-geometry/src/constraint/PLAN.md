# ifc-geometry constraint plan

Status: active scaffold under parent task(s) `GEOM-PLACE`.
Last updated: 2026-08-19

Follow `AGENTS.md`. Claim one local task, leave blockers/decisions beneath it,
and check it off only after the proof runs.

## Work queue

- [x] `CON-LOCAL` - placement links and relative-chain rules
  - Audited (2026-09-05): Placement links and relative-chain rules are implemented with cycle-safe resolution.
  - Proof: focused tests, crate clippy, and relevant declaration/corpus gate.
- [ ] `CON-GRID` - axes/intersections and dimensional checks
  - Audited (2026-09-05): grid axes and intersections are implemented and
    tested; the dimensional checks named in this task are not. No
    `DimensionCount`-style validation exists in `constraint/grid.rs`.
  - Proof: focused tests, crate clippy, and relevant declaration/corpus gate.
- [x] `CON-CONNECT` - connection geometry select coverage
  - Audited (2026-09-05): Connection-geometry select coverage is implemented and tested.
  - Proof: focused tests, crate clippy, and relevant declaration/corpus gate.
- [x] `CON-CYCLE` - adversarial cycles/depth budgets in resolver tests
  - Audited (2026-09-05): Adversarial cycle and depth-budget cases are covered in resolver tests.
  - Proof: focused tests, crate clippy, and relevant declaration/corpus gate.

## Completion log

Append `TASK-ID - proof - material decision`; keep long logs out of this file.
