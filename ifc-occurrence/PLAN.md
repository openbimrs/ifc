# ifc-occurrence implementation plan

Status: complete; the catalogue and writer are implemented.
Last updated: 2026-09-21

This is task state, not ambient context. Follow `AGENTS.md`; claim one task ID,
record blockers/decisions under it, and check it off only with evidence.

## Established boundary

The 141 concrete `IfcElement` occurrence classes, their `PredefinedType`
enums, and the type class each may be typed by.

## Planned file map

These paths are compiled private scaffold modules. Implement inside the named
owner and expose a public symbol only through an intentional parent re-export.

- `src/authoring.rs`: the writer and its refusals
- `src/table/mod.rs`: generated catalogue root and `ALL`
- `src/table/part1.rs`: generated catalogue shard
- `src/table/part2.rs`: generated catalogue shard
- `src/table/part3.rs`: generated catalogue shard
- `src/table/part4.rs`: generated catalogue shard
- `src/table/part5.rs`: generated catalogue shard

## Work queue

- [x] `OCC-TABLE` - generate the occurrence catalogue from the schema
  - Evidence: 141 entries, 122 with a paired type class, invariants asserted in tests.
- [x] `OCC-WRITE` - implement the writer and its five refusals
  - Evidence: `cargo test -p ifc-occurrence` 8 passed; 8/8 mutation probes.

## Completion log

Append concise entries as `TASK-ID - proof command/result - material decision`.
Do not paste long logs or duplicate standing rules from `AGENTS.md`.

- `OCC-TABLE` - 141 entries; `cargo test -p ifc-occurrence` 8 passed - The
  target list comes from concrete `IfcElement` subtypes that no crate
  authors, computed with test fixtures and doc examples excluded. An
  earlier scan counted `ifc-geometry`'s fixture builders as writers and
  wrongly reported `IfcWall` as already authored; the corrected count is
  141, not 140. `PredefinedType` sits at slot 8 for 121 classes but at 9,
  10, 12 or 17 for eleven others, and ten classes declare no enum at all,
  so the slot is read from the table rather than assumed.
- `OCC-WRITE` - 8/8 mutation probes - `CorrectTypeAssigned` is the rule
  worth enforcing at construction: it names exactly one permitted type
  class per occurrence, so the writer resolves `typed_by` against the
  model and refuses a mismatch. Two probes survived the first sweep, a
  blank `ObjectType` and a malformed `GlobalId`, both closed with tests.
