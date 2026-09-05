# ifc-geometry resource plan

Status: active scaffold under parent task(s) `GEOM-CENSUS`.
Last updated: 2026-08-19

Follow `AGENTS.md`. Claim one local task, leave blockers/decisions beneath it,
and check it off only after the proof runs.

## Work queue

- [x] `RES-SLOTS` - verify every accessor against generated absolute slots
  - Audited (2026-09-05): Every resource module declares explicit absolute-slot constants with inheritance order; 71 focused tests assert slot placement.
  - Proof: focused tests, crate clippy, and relevant declaration/corpus gate.
- [x] `RES-FUNC` - implement/test each EXPRESS helper or mark delegated/unsupported
  - Audited (2026-09-05): EXPRESS helpers are implemented or carry a named delegated/unsupported marker.
  - Proof: focused tests, crate clippy, and relevant declaration/corpus gate.
- [x] `RES-VIEW` - keep construction zero-copy and validation separate
  - Audited (2026-09-05): Views borrow via Slots and keep validation in separate accessors.
  - Proof: focused tests, crate clippy, and relevant declaration/corpus gate.
- [x] `RES-MANIFEST` - map every resource declaration to one owner
  - Audited (2026-09-05): tests/declaration_manifest.rs maps 163 declarations to one owner each and fails on drift.
  - Proof: focused tests, crate clippy, and relevant declaration/corpus gate.

## Completion log

Append `TASK-ID - proof - material decision`; keep long logs out of this file.
