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
  - Blocked on buildingSMART/IFC4.x-development#1193. That issue asks which
    reading is normative; IFC4.x-development#7 changed the factor but left the
    offset question open.
- [x] `UNIT-MEASURE` - audit the measures refused as unmapped
  - Mapped: `IfcSectionalAreaIntegralMeasure` → `SECTIONAREAINTEGRALUNIT`. The
    evidence is its type definition (m^5) and the IFC4 annex E
    `structural-curve-member` assignment of length^5.
  - Still refused, each for a stated reason:
    - `IfcThermalConductivityMeasure`: no release documents a unit enum for
      it, and `THERMALCONDUCTANCEUNIT` is only described as "Thermal
      Conductance unit".
    - `IfcMonetaryMeasure`: a currency, not an SI scale.
    - `IfcDescriptiveMeasure` and `IfcContextDependentMeasure`: they carry no
      unit semantics.
    - `IfcCompoundPlaneAngleMeasure`: a list.
    - The dB and pH measures: logarithmic.
  - Proof: `tests/exact_unit.rs::a_sectional_area_integral_uses_the_section_area_integral_unit`
    under IFC4 and IFC2X3.

## Completion log

`UNIT-SI..UNIT-CYCLE` - `cargo test -p ifc-properties` (`tests/exact_unit.rs`,
28 tests; `unit::si::tests` against both schemas); 8/8 hand mutants killed -
the traversal lives in `exact/` because it needs the release-bound record checks
there. `conversion.rs` and `derived.rs` remain owner scaffolds for permissive
views.
