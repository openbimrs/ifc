# ifc-spatial implementation plan

Status: active under `SPATIAL`. Last updated: 2026-08-26.
Follow `AGENTS.md`; claim one task and record blockers/decisions beneath it.

## Planned file map

These paths are implemented modules, not scaffold: each owns a tested contract
and is re-exported deliberately by its parent.

- `src/relation/slots.rs`: schema-fixed attribute positions
- `src/relation/link.rs`: reading a relationship's two ends
- `src/relation/index.rs`: reusable reverse-index-backed inverse queries
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
- [x] `SPATIAL-INV` - use `ifc-model`'s reverse index to answer inverse queries
  - `RelationshipIndex` borrows the model snapshot and builds one reusable
    `ReverseIndex`; repeated `naming` queries preserve tolerant decoding,
    endpoint semantics, and deterministic ordering without relationship rescans.
  - Proof: `cargo +1.88.0 test -p ifc-spatial --all-targets` (23 tests),
    strict all-target Clippy, and strict rustdoc pass. The public parity test
    kills a relationship-slot selection mutant.
- [ ] `SPATIAL-PSET` - group properties by container
  - `ifc-properties` is implemented; compose its borrowed views at an L4 seam
    without adding a sibling-crate dependency here.

## Completion log

`SPATIAL-TREE` - 31 tests, 9/9 mutants caught - slot layouts are constants
asserted against three schema versions rather than runtime lookups.
