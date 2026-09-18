# ifc-alignment instructions

Purpose: Interpret IFC4x3 alignment intent in both directions — read and lower
into exact neutral curves/frames, and author alignment records from plain
numbers (ADR 0011) — without meshing or backend selection.

Follow `../AGENTS.md`. Read `PLAN.md` only for assigned implementation or
roadmap work; keep progress, blockers, and evidence there.

## Boundary

Allowed production dependencies: ifc-model, schema metadata, and exact neutral axiolid-core/axiolid-curve representations; axiolid-model only if graph output is required.

## Module ownership

- `alignment.rs`: root/nesting/representation association
- `view.rs`: pins the IFC4X3 schema and exposes bounded `IfcRelNests`
  traversal shared by every layout module
- `horizontal.rs`: horizontal layout and segment parameters
- `vertical.rs`: vertical profile segments
- `cant.rs`: cant/superelevation segments and layout assembly
- `segment.rs`: shared segment transitions/continuity
- `curve.rs`: exact neutral curve assembly (single-segment and composite)
- `placement.rs`: linear placement/point-by-distance
- `referent.rs`: stationing/referents and station equations
- `query.rs`: bounded alignment traversal
- `curve/elevation.rs`: vertical segments as exact `ElevationLaw`s. Only
  CONSTANTGRADIENT and PARABOLICARC are polynomial in plan distance;
  CIRCULARARC is refused rather than approximated by a parabola.
- `curve/gradient.rs`: plan and profile composed as `Curve3::Elevated`.
  Height is a function of PLAN distance, not 3D arc length -- they diverge by
  sqrt(1 + g^2) wherever grade is non-zero.
- `slot.rs`: attribute slots read from the IFC4X3 ADD2 schema, indexed by both
  the readers and `authoring/`. Never restate a slot number elsewhere.
- `authoring/`: write direction. Plain `f64` in the file's declared units; no
  unit conversion (the readers apply `AlignmentUnits` on the way out) and no
  kernel types. Refuses what the readers reject, before staging.

## Invariants

- Preserve exact transition intent; tessellation is a downstream explicit operation.
- Station, horizontal length, projected length, and 3D length are distinct quantities.
- No road/rail product workflow or rendering policy enters this bridge.

Keep cross-resource projections attribute-scoped: shared `ifc-model` storage
does not make one feature crate the owner of an IFC entity. Split typed views,
resolution, lowering, mutation, and validation before they grow together.

## Verification

Run targeted tests/clippy, isolated build, and the package architecture/context
gates. Geometry bridges also run declaration/corpus coverage and the full gate.

## Station for a horizontal segment

An `IfcAlignmentHorizontalSegment` does not state its own distance along:
position follows from chaining. A Viennese bend needs that station to read
the cant swing across itself, so both chain walks in `curve/assemble.rs`
accumulate it.

Two pitfalls, both mutation-tested:

- The partial walk `continue`s on a refused segment. Advance the station
  BEFORE that branch or every later segment is mis-stationed.
- There are TWO accumulators, one per walk. A test that only exercises
  `lower_horizontal_layout` leaves the partial walk unguarded.

A single-segment lowering has no chain context, so it refuses a Viennese
bend rather than passing a station of zero.
