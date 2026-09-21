# ifc-occurrence instructions

Purpose: authoring the 141 concrete `IfcElement` occurrence classes - the
built elements and distribution equipment that appear in a model, as
distinct from the type definitions they borrow their description from.

Follow `../AGENTS.md`. Read `PLAN.md` only when assigned implementation or
roadmap work; record progress, blockers, and evidence there, not here.

## Boundary

Allowed production dependencies: `ifc-model` only. This crate holds no
geometry, no placement, and no spatial containment: an occurrence is
staged here, then placed and contained by the crates that own those
concerns.

## The catalogue is generated

`src/table/` is written by `scripts/gen-occurrences.py` from the
IFC4X3 ADD2 EXPRESS schema. Do not hand-edit it. Regenerate after a
schema bump and re-run the crate tests, which assert the catalogue's
invariants rather than trusting generation.

Shards are sized by type count, not letter range: `cargo fmt` expands
the generated literals to roughly 1.7x their emitted line count, and the
workspace fails any source file over 800 lines.

## What the writer refuses

- a `PredefinedType` token the entity's own enum does not declare
- `USERDEFINED` without a non-blank `ObjectType` to name it
- a `typed_by` reference whose entity is not the one type class
  `CorrectTypeAssigned` permits for that occurrence
- a `PredefinedType` on the ten classes that declare no enum
- a `GlobalId` that is not a 22-character IFC GUID

The type pairing is the load-bearing one: 122 of the 141 classes name
exactly one permitted type class, so a pump typed by a valve type is a
schema violation this crate will not write.
