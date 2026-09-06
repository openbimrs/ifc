# ifc-geometry rules plan

Status: active scaffold under parent task(s) `GEOM-CENSUS`.
Last updated: 2026-08-19

Follow `AGENTS.md`. Claim one local task, leave blockers/decisions beneath it,
and check it off only after the proof runs.

## Work queue

- [x] `RULE-REG` - inventory all relevant WHERE rules and support state
  - Done (2026-09-05): `data/ifc4-where-rules.tsv` lists all 95 geometry-
    resource rules across 56 entities, each `implemented` or `inventoried`.
    `tests/where_rule_inventory.rs` regenerates it from the bundled schema
    and fails on drift in either direction. Needs openbim-step >= 0.5.0,
    which captures `EntityDef::where_rules`.
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
