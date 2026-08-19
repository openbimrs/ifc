# ifc-properties implementation plan

Status: architecture scaffold; quantity/property views and mutation APIs remain to implement.
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

- [ ] `PROP-PSET` - implement all property value families as borrowed views
  - Evidence: focused unit/property/fixture tests, isolated build, and crate clippy.
- [ ] `PROP-QTY` - implement authored simple/complex quantity views
  - Evidence: focused unit/property/fixture tests, isolated build, and crate clippy.
- [ ] `PROP-UNIT` - implement dimensional unit resolution shared by properties/quantities
  - Evidence: focused unit/property/fixture tests, isolated build, and crate clippy.
- [ ] `PROP-TEMPLATE` - implement templates and applicability links
  - Evidence: focused unit/property/fixture tests, isolated build, and crate clippy.
- [ ] `PROP-QUERY` - resolve occurrence/type property assignment with precedence made explicit
  - Evidence: focused unit/property/fixture tests, isolated build, and crate clippy.
- [ ] `PROP-EDIT` - write/update quantities transactionally after MODEL-MUT
  - Evidence: focused unit/property/fixture tests, isolated build, and crate clippy.
- [ ] `PROP-CHECK` - accept externally computed measurements and compare without depending on geometry
  - Evidence: focused unit/property/fixture tests, isolated build, and crate clippy.

## Completion log

Append concise entries as `TASK-ID - proof command/result - material decision`.
Do not paste long logs or move standing invariants out of `AGENTS.md`.
