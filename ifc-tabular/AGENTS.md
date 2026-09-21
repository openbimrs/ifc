# ifc-tabular instructions

Purpose: structured IFC value containers indexed by position or time —
`IfcTable` with its rows and columns, and `IfcTimeSeries` with its value
records.

Follow `../AGENTS.md`. Read `PLAN.md` only for assigned implementation or
roadmap work; keep progress, blockers, and evidence there.

## Why this crate exists

These entities carry no domain meaning. A table is values indexed by row
and column; a time series is values indexed by time. Both bottom out in
`LIST OF IfcValue`, and the schema itself groups them: `IfcMetricValueSelect`
and `IfcObjectReferenceSelect` each name `IfcTable` and `IfcTimeSeries`
together.

Their consumers are spread across constraints, cost, and resources. Housing
them in any one of those crates would make the other two reach into it, so
they live here and are referenced by `EntityId`.

This is NOT a bucket for unrelated leftovers. A new entity belongs here only
if it is a container of `IfcValue` indexed by position or time.

## Boundary

Allowed production dependencies: `ifc-model`, `ifc-schema`, `thiserror`.
No sibling domain crate, no codec, no geometry.

`MeasureValue` lives in `ifc-properties` and stays there: siblings do not
depend on one another. Cells are therefore taken as `ifc_model::Value`,
which is also why this crate never invents a measure wrapper — the caller
decides between `4.2` and `IFCLENGTHMEASURE(4.2)`.

## Invariants

- WR1: every row carries the same cell count as the first row.
- WR2: at most one row is a heading.
- Every `LIST [1:?]` refuses an empty list rather than writing one.
- Blank required text is refused: it satisfies EXISTS while naming nothing.
- `IfcTable`'s three DERIVE attributes introduce NEW names, so they occupy
  no instance slots and are never written. Contrast `IfcSIUnit`, whose
  DERIVE redeclares an inherited attribute and does take a slot, written
  as `*`. `tests/slot_layout.rs` holds this line against the schema.
- Row widths are passed in alongside row ids because a staged entity cannot
  be read back out of a `Transaction`, and WR1 is stated over cell counts.

