# IFC test fixtures plan

Status: curated standalone fixture corpus used by IFC codec, validation, and geometry tests.
Last updated: 2026-08-25

This is task state, not ambient context. Follow `AGENTS.md` and load this file
only when changing the fixture corpus.

## Established boundary

Small synthetic or minimal upstream IFC files with preserved provenance.

## Planned file map

- `ifclite-geometry/`: geometry and processing edge cases.
- `ifcopenshell-validate/`: schema/header validation cases.
- `costing/`: local costing round-trip fixture.

## Work queue

- [ ] `FIXTURE-PROVENANCE` - add a machine-checkable source and license manifest for every fixture
  - Evidence: manifest coverage test maps every `.ifc` file to provenance and license metadata.

## Completion log

Append concise entries as `TASK-ID - proof command/result - material decision`.

## `synthetic-compile/` — authored here, not upstream

`union_over_halfspace_unbounded.ifc`, derived from
`ifclite-geometry/issue_1155_halfspace_flyaway.ifc` by retyping
`IFCBOOLEANCLIPPINGRESULT(.DIFFERENCE.,...)` to `IFCBOOLEANRESULT(.UNION.,...)`.
Not byte-identical to upstream, so it must never move into
`ifclite-geometry/`, whose provenance rule requires exactly that.

Difference and intersection stay inside the finite left operand, so a prism
covering its bounds is an exact stand-in -- which is why `issue_1155`
compiles. Union escapes that bound and the reference compiler refuses.
The file validates clean: the refusal belongs to the mesh provider, not the
file. It keeps the refusal branch of `ifc-geometry/tests/compile_pairing.rs`
executable; a mutation run confirmed that branch is dead code without it.
