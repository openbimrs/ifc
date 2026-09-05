# ifc-geometry rules plan

Status: active scaffold under parent task(s) `GEOM-CENSUS`.
Last updated: 2026-08-19

Follow `AGENTS.md`. Claim one local task, leave blockers/decisions beneath it,
and check it off only after the proof runs.

## Work queue

- [ ] `RULE-REG` - inventory all relevant WHERE rules and support state
  - Audited (2026-09-05): placement and solid rules are implemented and
    tested, but there is no executable rule inventory - no data file lists the
    schema's WHERE rules with a support state, so coverage cannot be proven
    the way `declaration_manifest.rs` proves declaration coverage.
  - Proof: focused tests, crate clippy, and relevant declaration/corpus gate.
- [x] `RULE-PLACE` - placement/direction dimensional rules
  - Audited (2026-09-05): Placement and direction dimensional rules are implemented and tested.
  - Proof: focused tests, crate clippy, and relevant declaration/corpus gate.
- [x] `RULE-SOLID` - swept/boolean/half-space rules
  - Audited (2026-09-05): Swept, boolean and half-space rules are implemented and tested.
  - Proof: focused tests, crate clippy, and relevant declaration/corpus gate.
- [x] `RULE-PROP` - pair every violation test with a conforming edge case
  - Audited (2026-09-05): Each violation test is paired with a conforming edge case.
  - Proof: focused tests, crate clippy, and relevant declaration/corpus gate.
- [x] `RULE-REPORT` - unsupported vs failed vs passed are distinct
  - Audited (2026-09-05): Unsupported, failed and passed are distinct variants and asserted as such.
  - Proof: focused tests, crate clippy, and relevant declaration/corpus gate.

## Completion log

Append `TASK-ID - proof - material decision`; keep long logs out of this file.
