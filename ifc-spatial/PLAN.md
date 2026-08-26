# ifc-spatial implementation plan

Status: active under `SPATIAL`. Last updated: 2026-08-26.
Follow `AGENTS.md`; claim one task and record blockers/decisions beneath it.

## Planned file map

These paths are implemented modules, not scaffold: each owns a tested contract
and is re-exported deliberately by its parent.

- `src/relation/slots.rs`: schema-fixed attribute positions
- `src/relation/link.rs`: reading a relationship's two ends
- `src/tree/kind.rs`: spatial role classification
- `src/tree/build.rs`: tree assembly and queries

## Work queue

- [x] `SPATIAL-REL` - read aggregation, containment and nesting relationships
  - Proof: `cargo test -p ifc-spatial`; slot constants asserted against IFC2x3,
    IFC4 and IFC4x3 in `tests/slot_layout.rs`.
- [x] `SPATIAL-TREE` - assemble the containment tree, tolerating real files
  - Proof: 12 tests covering omitted levels, elements on the building,
    duplicate storeys, dangling references, cycles and element decomposition.
- [x] `SPATIAL-REAL` - verify against exporter output
  - Proof: `tests/real_files.rs`. Found a corpus file that uses `IfcRelAggregates`
    exclusively with **no** containment relationship; the tree handles it and
    the case is now pinned.
- [ ] `SPATIAL-INV` - use `ifc-model`'s reverse index to answer inverse queries
  - Currently `relation::naming` rescans relationships per call. Fine for a
    single query, wasteful in a loop. Needs a borrowed index type so callers
    opt into building it once.
- [ ] `SPATIAL-PSET` - group properties by container
  - Blocked: `ifc-properties` is scaffold, so there is nothing to group yet.

## Completion log

`SPATIAL-TREE` - 31 tests, 9/9 mutants caught - slot layouts are constants
asserted against three schema versions rather than runtime lookups.
