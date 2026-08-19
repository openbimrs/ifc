# ifc-geometry select plan

Status: active scaffold under parent task(s) `GEOM-CENSUS`.
Last updated: 2026-08-19

Follow `AGENTS.md`. Claim one local task, leave blockers/decisions beneath it,
and check it off only after the proof runs.

## Work queue

- [ ] `SEL-MANIFEST` - regenerate/check subtype table against authoritative schema
  - Proof: focused tests, crate clippy, and relevant declaration/corpus gate.
- [ ] `SEL-POSNEG` - pair every family with valid and invalid membership tests
  - Proof: focused tests, crate clippy, and relevant declaration/corpus gate.
- [ ] `SEL-VERS` - make schema-version identity explicit before IFC4x3 support
  - Proof: focused tests, crate clippy, and relevant declaration/corpus gate.

## Completion log

Append `TASK-ID - proof - material decision`; keep long logs out of this file.
