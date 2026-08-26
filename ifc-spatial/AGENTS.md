# ifc-spatial instructions

Scope: IFC containment and objectified relationship traversal. Follow the
package `../AGENTS.md`. Read `PLAN.md` only for assigned task(s) `SPATIAL` and
keep implementation state there.

## Owns

- classifying an entity's spatial role by type name
- reading `IfcRelAggregates`, `IfcRelContainedInSpatialStructure`, `IfcRelNests`
- assembling the project/site/building/storey/element tree
- reporting containment anomalies: orphaned containers, dangling references

## Does not own

- validation: cardinality and WHERE rules belong to `ifc-validate`; this crate
  reports what the file says and never rejects it
- geometry, properties, or any other interpretation of the elements it groups
- generic graph machinery: budgets, walks and the reverse index live in
  `ifc-model` because they carry no domain meaning

## Boundaries

L2. Depends on `ifc-model` (L0) only. It does **not** depend on `ifc-schema`:
the slot layouts it needs are fixed across IFC2x3/IFC4/IFC4x3, so requiring a
parsed `.exp` file at runtime would be a cost with no benefit. That trade is
only safe because `tests/slot_layout.rs` asserts the constants against all
three shipped schemas — do not add a slot constant without extending it.

## Pitfall

`IfcRelAggregates` and `IfcRelContainedInSpatialStructure` use **opposite**
slot orders for their relating/related ends. Assuming a uniform layout inverts
the tree silently and every downstream answer is wrong. See `src/relation/slots.rs`.
