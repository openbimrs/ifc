# ifc-properties unit plan

Status: exact resolution implemented under `PROP-UNIT` (#53). Last updated: 2026-09-26.
Follow `AGENTS.md`; claim one task and record blockers/decisions beneath it.

## Work queue

- [x] `UNIT-SI` - SI prefix and dimensions
  - `si.rs` holds both releases' `IfcDimensionsForSiUnit` and
    `IfcCorrectDimensions` tables. A unit test re-parses both from
    `references/ifc-spec`. The prefix applies before the power (mm² = 1e-6).
    An unknown prefix is `None`, not 0.
- [x] `UNIT-CONV` - conversion-based units/chains
  - Resolved in `exact/unit/resolve.rs` as factor × `UnitComponent`,
    recursively. `IfcConversionBasedUnitWithOffset` is refused: IFC4's
    definition ("offset added after the factor") contradicts its own Fahrenheit
    example (factor 1.8, `f = k·1.8 − 459.67`). The permissive `UnitKind` keeps
    the raw offset.
- [x] `UNIT-DERIVED` - derived unit elements/dimensions
  - Product of element scales and exponent-weighted dimensions. An element
    with an offset (e.g. `DEGREE_CELSIUS`) is refused.
- [x] `UNIT-ASSIGN` - project defaults and local override resolution
  - `exact_unit` uses the explicit unit, otherwise the single `IfcProject`'s
    `UnitsInContext`. Two units of the needed type are refused.
- [x] `UNIT-CYCLE` - conversion cycle/budget tests
  - Cycles are reported with their members. Depth is bounded by
    `Budget::DEFAULT.max_depth`.
- [ ] `UNIT-OFFSET` - apply `ConversionOffset` once buildingSMART resolves
  the direction contradiction above.
- [ ] `UNIT-MEASURE` - measures refused as unmapped because their unit enum is
  spelt differently (`IfcThermalConductivityMeasure`) or they are not a
  linear scalar (monetary, logarithmic, compound angle). Each needs its own
  cited correspondence.

## Completion log

`UNIT-SI..UNIT-CYCLE` - `cargo test -p ifc-properties` (`tests/exact_unit.rs`,
28 tests; `unit::si::tests` against both schemas); 8/8 hand mutants killed -
the traversal lives in `exact/` because it needs the release-bound record checks
there. `conversion.rs` and `derived.rs` remain owner scaffolds for permissive
views.
