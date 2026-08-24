# ifc-geometry rules plan

Status: active scaffold under parent task(s) `GEOM-CENSUS`.
Last updated: 2026-08-19

Follow `AGENTS.md`. Claim one local task, leave blockers/decisions beneath it,
and check it off only after the proof runs.

## Work queue

- [ ] `RULE-REG` - inventory all relevant WHERE rules and support state
  - Proof: focused tests, crate clippy, and relevant declaration/corpus gate.
- [ ] `RULE-PLACE` - placement/direction dimensional rules
  - Proof: focused tests, crate clippy, and relevant declaration/corpus gate.
- [ ] `RULE-SOLID` - swept/boolean/half-space rules
  - Proof: focused tests, crate clippy, and relevant declaration/corpus gate.
- [ ] `RULE-PROP` - pair every violation test with a conforming edge case
  - Proof: focused tests, crate clippy, and relevant declaration/corpus gate.
- [ ] `RULE-REPORT` - unsupported vs failed vs passed are distinct
  - Proof: focused tests, crate clippy, and relevant declaration/corpus gate.

## Completion log

Append `TASK-ID - proof - material decision`; keep long logs out of this file.
