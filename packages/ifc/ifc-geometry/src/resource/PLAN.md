# ifc-geometry resource plan

Status: active scaffold under parent task(s) `GEOM-CENSUS`.
Last updated: 2026-08-19

Follow `AGENTS.md`. Claim one local task, leave blockers/decisions beneath it,
and check it off only after the proof runs.

## Work queue

- [ ] `RES-SLOTS` - verify every accessor against generated absolute slots
  - Proof: focused tests, crate clippy, and relevant declaration/corpus gate.
- [ ] `RES-FUNC` - implement/test each EXPRESS helper or mark delegated/unsupported
  - Proof: focused tests, crate clippy, and relevant declaration/corpus gate.
- [ ] `RES-VIEW` - keep construction zero-copy and validation separate
  - Proof: focused tests, crate clippy, and relevant declaration/corpus gate.
- [ ] `RES-MANIFEST` - map every resource declaration to one owner
  - Proof: focused tests, crate clippy, and relevant declaration/corpus gate.

## Completion log

Append `TASK-ID - proof - material decision`; keep long logs out of this file.
