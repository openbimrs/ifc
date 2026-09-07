# ifc-spatial instructions

Scope: IFC containment and objectified relationship traversal. Follow the
package `../AGENTS.md`. Read `PLAN.md` only for assigned task(s) `SPATIAL` and
keep implementation state there.

## Owns

- classifying an entity's spatial role by type name
- reading `IfcRelAggregates`, `IfcRelContainedInSpatialStructure`, `IfcRelNests`
- reading `IfcRelSpaceBoundary` and its `1stLevel`/`2ndLevel` subtypes: which
  element bounds a space, with parent/corresponding links
- reading `IfcRelCoversBldgElements` and `IfcRelCoversSpaces`: finishes on an
  element and finishes bounding a space, kept apart because the same covering
  can do both and the two answer different questions
- reading `IfcRelConnectsElements` with its two subtypes, and
  `IfcRelInterferesElements`. The connects family puts `ConnectionGeometry` at
  slot 4 so its ends are at **5/6**; interference is not a subtype of it and
  keeps **4/5**
- reading the `IfcRelAssigns*` families this crate owns: to actor, process,
  product and group-by-factor. Their ends BRACKET `RelatedObjectsType`:
  related at **4**, the enum at 5, relating at **6**
- reading `IfcRelDeclares`, `IfcRelDefinesByObject`,
  `IfcRelFlowControlElements`, `IfcRelServicesBuildings` and
  `IfcRelConnectsWithEccentricity`. These five disagree on layout: three are
  relating-first, two are related-first
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

`Model::ids_of_type` matches an EXACT type name. `IfcRelSpaceBoundary` has two
concrete subtypes and real BEM exports write the deepest one, so all three are
queried explicitly in `src/relation/slots.rs`; querying the supertype alone
returns an empty list rather than an error.

`IfcRelAggregates` and `IfcRelContainedInSpatialStructure` use **opposite**
slot orders for their relating/related ends. Assuming a uniform layout inverts
the tree silently and every downstream answer is wrong. See `src/relation/slots.rs`.
