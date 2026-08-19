# ifc-material layer plan

Status: planned under `MAT-LAYER`. Last updated: 2026-08-19.
Follow `AGENTS.md`; claim one task and record blockers/decisions beneath it.

## Work queue

- [ ] `LAYER-VIEW` - identity, material link, metadata, and authored thickness
  - Proof: absolute-slot, invalid-thickness, reference, and crate-clippy tests.
- [ ] `LAYER-SET` - ordered semantic membership and total authored thickness
  - Requires: `LAYER-VIEW`.
  - Proof: order/empty-set/aggregate and crate-clippy tests.
- [ ] `LAYER-USAGE` - semantic association from usage to layer set only
  - Requires: `LAYER-SET`.
  - Proof: association/cycle and crate-clippy tests; no geometry-slot accessors.
- [ ] `LAYER-CROSS` - shared fixture with geometry's material-usage projection
  - Requires: `LAYER-USAGE`, `INPUT-MAT`.
  - Proof: both projections join by EntityId without crate dependencies or duplicate slot parsing.

## Completion log

Append `TASK-ID - proof - material decision`; no long logs.
