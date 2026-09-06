# ifc-geometry constraint plan

Status: active scaffold under parent task(s) `GEOM-PLACE`.
Last updated: 2026-08-19

Follow `AGENTS.md`. Claim one local task, leave blockers/decisions beneath it,
and check it off only after the proof runs.

## Work queue

- [x] `CON-LOCAL` - placement links and relative-chain rules
  - Audited (2026-09-05): Placement links and relative-chain rules are implemented with cycle-safe resolution.
  - Proof: focused tests, crate clippy, and relevant declaration/corpus gate.
- [x] `CON-GRID` - axes/intersections and dimensional checks
  - Implemented (2026-09-05): the dimensional checks this task named are now
    in `rules::grid`, which checks both rules the schema declares for
    `IfcGridAxis`. `WR1` keeps an axis curve 2D; `WR2` keeps an axis in
    exactly one U/V/W list, counted from the owning grid because a Part 21
    file carries no inverse attributes.
  - Proof: four focused tests including the silent conforming case, and 2/2
    mutation probes (accepting any dimension, accepting any membership count).
  - Proof: focused tests, crate clippy, and relevant declaration/corpus gate.
- [x] `CON-CONNECT` - connection geometry select coverage
  - Audited (2026-09-05): Connection-geometry select coverage is implemented and tested.
  - Proof: focused tests, crate clippy, and relevant declaration/corpus gate.
- [x] `CON-CYCLE` - adversarial cycles/depth budgets in resolver tests
  - Audited (2026-09-05): Adversarial cycle and depth-budget cases are covered in resolver tests.
  - Proof: focused tests, crate clippy, and relevant declaration/corpus gate.

## Completion log

Append `TASK-ID - proof - material decision`; keep long logs out of this file.
