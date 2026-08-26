# 0008 — Fixed slot constants for stable relationships

- **Status:** Accepted
- **Date:** 2026-08-26
- **Deciders:** openbimrs contributors
- **Supersedes:** —

## Context

IFC stores no parent pointers. A wall does not name its storey; a separate
`IfcRelContainedInSpatialStructure` entity names both ends. So every containment
question is a question about relationship entities and the attribute slots their
ends sit in.

Those two slots are not in a consistent order:

```text
IfcRelAggregates                    4 = RelatingObject   5 = RelatedObjects
IfcRelContainedInSpatialStructure   4 = RelatedElements  5 = RelatingStructure
```

Reading containment as though it matched aggregation inverts the tree: elements
become the parents of their storey. Nothing about that failure is loud — the
types match, the walk terminates, and every downstream answer is wrong.

The obvious safeguard is to resolve slots through `ifc-schema` at runtime, the
way [ADR 0007](/adr/0007-authoring-is-a-schema-layer-not-a-model-layer) does for
authoring. That would make containment traversal — the most common query any
consumer makes — depend on parsing a 3 MB EXPRESS file, to answer a question
whose answer is fixed for the lifetime of the schema.

## Decision

`ifc-spatial` states the slot positions as constants and does **not** depend on
`ifc-schema`.

The constants are asserted against the shipped IFC2x3, IFC4 and IFC4x3 schemas
in `tests/slot_layout.rs`, including an explicit test that the two relationships
still disagree. A slot constant may not be added without extending that test.

## Consequences

- Containment traversal works on a model alone; no schema file, no `schema`
  feature, no parse cost.
- The correctness argument moves from the type system into a test. That is only
  acceptable because the test runs against the normative artifacts rather than
  against a restatement of the same assumption.
- A future schema that reorders these attributes fails the layout test rather
  than silently inverting a customer's floor plan.
- Relationships whose layout is *not* stable across versions must not be added
  as constants. They belong behind a schema lookup, and the boundary note in
  that crate's **AGENTS.md** says so.

## Alternatives considered

| Option | Why not |
| --- | --- |
| Resolve every slot via `ifc-schema` | Forces a schema parse on the most common query in the library to answer a question that cannot change within a schema version |
| Hard-code without the layout test | The failure mode is silent inversion; an untested constant is the whole risk |
| Put traversal in `ifc-model` | `ifc-model` must not name concrete entity types — enforced by `model_invariants.rs` |
