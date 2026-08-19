# ifc-material profile plan

Status: planned under `MAT-PROFILE`. Last updated: 2026-08-19.
Follow `AGENTS.md`; claim one task and record blockers/decisions beneath it.

## Work queue

- [ ] `MATPROF-DEF` - material/name/description/priority/category projection
  - Proof: absolute-slot, invalid-reference, and crate-clippy tests.
- [ ] `MATPROF-SET` - ordered semantic membership and composite indicator
  - Requires: `MATPROF-DEF`.
  - Proof: order/empty-set/invalid-reference and crate-clippy tests.
- [ ] `MATPROF-USAGE` - semantic association from usage to profile set only
  - Requires: `MATPROF-SET`.
  - Proof: association/cycle and crate-clippy tests; no geometry-slot accessors.
- [ ] `MATPROF-CROSS` - shared fixture with geometry's material-usage projection
  - Requires: `MATPROF-USAGE`, `INPUT-MAT`.
  - Proof: both projections join by EntityId without crate dependencies or duplicate slot parsing.

## Completion log

Append `TASK-ID - proof - material decision`; no long logs.
