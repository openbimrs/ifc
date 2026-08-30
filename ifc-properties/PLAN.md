# ifc-properties implementation plan

Status: implemented; every task in the work queue is complete.
Last updated: 2026-08-19

This is task state, not ambient context. Follow `AGENTS.md`; claim one task ID,
record blockers/decisions under it, and check it off only with evidence.

## Established boundary

Borrowed property, quantity, unit, template, and standard-library projections plus model authoring ports.

## Planned file map

These paths are compiled private scaffold modules. Implement inside the named
owner and expose a public symbol only through an intentional parent re-export.

- `src/pset/set.rs`: IfcPropertySet and relationships
- `src/pset/scalar.rs`: single/bounded/list/enumerated values
- `src/pset/table.rs`: table values and interpolation metadata
- `src/pset/reference.rs`: object/reference properties
- `src/pset/complex.rs`: nested complex properties
- `src/quantity/set.rs`: IfcElementQuantity
- `src/quantity/simple.rs`: length/area/volume/count/time/weight
- `src/quantity/complex.rs`: nested physical complex quantities
- `src/quantity/edit.rs`: transactional authored quantity updates
- `src/quantity/validation.rs`: units/dimensions/formula consistency
- `src/unit/assignment.rs`: project unit context
- `src/unit/si.rs`: SI prefixes/dimensions
- `src/unit/conversion.rs`: conversion-based units
- `src/unit/derived.rs`: derived dimensions/elements
- `src/template/property_set.rs`: set templates
- `src/template/property.rs`: property templates
- `src/query/assignment.rs`: object/type set assignment

- `src/pset/aggregate.rs`: compiled private scaffold; implementation owned by `src/pset/PLAN.md`
- `src/template/relationship.rs`: compiled private scaffold; implementation owned by `src/template/PLAN.md`
- `src/unit/monetary.rs`: compiled private scaffold; implementation owned by `src/unit/PLAN.md`

## Work queue

- [x] `PROP-PSET` - implement all property value families as borrowed views
  - Evidence: focused unit/property/fixture tests, isolated build, and crate clippy.
- [x] `PROP-QTY` - implement authored simple/complex quantity views
  - Evidence: focused unit/property/fixture tests, isolated build, and crate clippy.
- [x] `PROP-UNIT` - implement dimensional unit resolution shared by properties/quantities
  - Evidence: focused unit/property/fixture tests, isolated build, and crate clippy.
- [x] `PROP-TEMPLATE` - implement templates and applicability links
  - Evidence: focused unit/property/fixture tests, isolated build, and crate clippy.
- [x] `PROP-QUERY` - resolve occurrence/type property assignment with precedence made explicit
  - Evidence: focused unit/property/fixture tests, isolated build, and crate clippy.
- [x] `PROP-EDIT` - write/update quantities transactionally after MODEL-MUT
  - Evidence: focused unit/property/fixture tests, isolated build, and crate clippy.
- [x] `PROP-CHECK` - accept externally computed measurements and compare without depending on geometry
  - Evidence: focused unit/property/fixture tests, isolated build, and crate clippy.

## Completion log

Append concise entries as `TASK-ID - proof command/result - material decision`.
Do not paste long logs or move standing invariants out of `AGENTS.md`.

PROP-PSET - cargo test -p ifc-properties (19 passing) - IfcPropertyBoundedValue
  states UpperBoundValue at slot 2 and LowerBoundValue at slot 3; reading them
  in the intuitive order inverts every range and still looks plausible.
  CurveInterpolation is an ENUM (.LINEAR.), not text: a text reader silently
  reports "unstated".
PROP-QTY - cargo test -p ifc-properties (19 passing) - simple quantity values
  live at slot 3 because Unit occupies slot 2 on IfcPhysicalSimpleQuantity;
  IfcPhysicalComplexQuantity has no Unit so its contents start at slot 2. WR21
  (unit type matches quantity kind) and WR22 (value >= 0) are reported as
  anomalies: ifcopenshell.validate does not check them.
PROP-UNIT - cargo test -p ifc-properties (19 passing) - SI prefixes are carried
  as decimal EXPONENTS, not factors: MILLI is 1e-3 exactly as a decimal and
  f64 cannot hold it, so the caller applies one multiplication instead of a
  chain of roundings. IfcDerivedUnit is NOT an IfcNamedUnit and its slots start
  at 0, so it cannot share a reader with IfcSIUnit.
PROP-TEMPLATE - cargo test -p ifc-properties (19 passing) - templates describe
  what a set should contain and carry no values, so they are read separately
  from psets rather than as a set variant.
PROP-QUERY - cargo test -p ifc-properties (19 passing) - occurrence overrides
  type by set NAME; the shadowed type set is retained so a checker can explain
  why an effective value differs from the type default. IfcRelDefinesByType
  puts objects at slot 4 and the type at slot 5, the OPPOSITE roles to
  IfcRelDefinesByProperties' same two slot numbers.
PROP-CHECK - cargo test -p ifc-properties (19 passing) - comparison takes a
  caller-computed value and never opens geometry; unit differences are
  REPORTED rather than converted, because a silent mm/m conversion is how a
  1000x error passes a check.

PROP-EDIT - cargo test -p ifc-properties (25 passing) - helpers STAGE onto a
caller-owned `Transaction` and never commit, so a takeoff spanning many
elements lands atomically or not at all. Writes preserve the declared measure
type and refuse a non-quantity target before staging, so a rejected edit never
reaches the batch. `IfcCountMeasure` is written as an INTEGER per the schema.
