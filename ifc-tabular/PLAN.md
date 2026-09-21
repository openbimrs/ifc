# ifc-tabular implementation plan

Status: authoring implemented for tables and time series; no read views yet.
Last updated: 2026-09-21

This is task state, not ambient context. Follow `AGENTS.md`; claim one task ID,
record blockers/decisions under it, and check it off only with evidence.

## Bounded contract

Transaction-staged authoring for `IfcTable`, `IfcTableRow`, `IfcTableColumn`,
`IfcRegularTimeSeries`, `IfcIrregularTimeSeries`, `IfcTimeSeriesValue` and
`IfcIrregularTimeSeriesValue`, with the WHERE rules those entities state.
This does not imply interpolation, resampling, unit conversion, or any
reading of what the values mean.

## Planned file map

- `src/lib.rs`: crate contract and re-exports
- `src/error.rs`: typed staging refusals
- `src/table.rs`: table, row and column staging with WR1/WR2
- `src/series.rs`: time-series staging and value records

- `tests/authoring.rs`: WHERE rules, slot placement, and measure passthrough
- `tests/slot_layout.rs`: hard-coded arities checked against the schema

## Work queue

- [x] `TAB-TABLE` - table, row and column staging with WR1 and WR2
- [x] `TAB-SERIES` - regular and irregular series plus their value records
- [x] `TAB-LAYOUT` - arities and derived-attribute absence pinned to the schema
- [ ] `TAB-READ` - borrowed projections for reading tables and series back

## Completion log

- `TAB-TABLE/SERIES/LAYOUT` - 11 tests pass across two suites: WR1 ragged
  refusal, WR2 heading limit, empty-list and blank-label refusals, rows and
  columns in their own slots, and cell measure wrappers preserved verbatim.
- Mutation proof - 5/5 killed on a green baseline, including derived counts
  written as slots and rows swapped with columns.
- Schema proof - `tests/slot_layout.rs` asserts all seven arities against
  IFC4 and IFC4X3, and proves `IfcTable`'s three DERIVE names occupy no
  instance slots.

