# 0023 — Derived indexes are maintained once built

- **Status:** Accepted
- **Date:** 2026-10-28
- **Deciders:** openbimrs contributors
- **Supersedes:** —

## Context

Staging a checked authoring batch cost time that grew with the size of the
model and, for a batch, with the square of its length
([#587](https://github.com/openbimrs/ifc/issues/587)). An editor authoring
one wall into a model of 100,000 walls (1.3 million entities) waited 727 ms
for it; a batch of 10,000 walls took 48 s and its undo 2.7 s; typing 10,000
walls took 30 s. The causes were scans repeated for every operation: the
planner read every `IfcRoot` of the model to find a GlobalId, every lookup of
a relationship scanned all relationships of its type, the overlay built the
whole reverse index, a commit that removed anything built it again, and an
undo searched file order from the front. Inside a batch, every operation
walked the batch's own records again, and every object typed by one type
copied and rechecked the type's whole, growing, list.

Reading a file must not pay for any of this: a codec that parses and writes
back never asks who references whom, which is why the reverse index was built
on demand and not on insert.

## Decision

The model keeps its derived indexes once something asks for them, and keeps
them current.

- `Model::reverse_index` and `Model::global_id_candidates` build their index
  on the first call, one scan, and every primitive write (`insert`,
  `set_attribute(s)`, `remove`) and every step of an undo or redo then moves
  only the difference between the footprint of the entity before and after
  (its references by slot, and its GlobalId text). A model nobody asks never
  builds them; `insert_lazy` drops them rather than decode;
  `Model::drop_derived_indexes` frees them. A clone copies them.
- The GlobalId index is schema-free: it lists every entity whose first
  attribute is text that `Guid::parse` accepts, which is where IFC puts the
  GlobalId of every `IfcRoot`. The authoring planner filters the candidates
  by `IfcRoot` with the schema. Text that does not parse is found by a scan,
  as before.
- The authoring overlay keeps two indexes over the batch's own records (who
  references a target, which created entities have a type) and reads the
  base model's reverse index. Relationship lookups go through the referrers
  of the object. The first object a batch assigns to a type's relationship
  rechecks the whole record; later ones check themselves and are appended.
- File order and type buckets are searched from the end, which is where an
  undo removes.

Every public signature of the authoring API is unchanged.

## Alternatives considered

| Option | Why not |
| --- | --- |
| An `AuthoringIndex` the caller holds and passes in | Threads a new argument through every call, and goes stale at every undo, history walk, property edit and direct write the caller makes without telling it. |
| Build the indexes eagerly on insert | Taxes every codec read for a query most reads never make. |
| Make the GlobalId index schema-aware | `ifc-model` takes no schema; the slot-0 convention plus a filter in the facade keeps the dependency direction. |

## Consequences

**Positive**

- Staging a batch costs the batch. Measured on one machine
  ([`benchmarks/baseline.md`](https://github.com/openbimrs/ifc/blob/main/benchmarks/baseline.md#authoring-587)):
  one wall into 100,000 walls 727 ms to 0.06 ms; a batch of 10,000 walls 48 s
  to 0.76 s (100,000 walls: 9.5 s); typing 10,000 walls 30 s to 0.12 s; the
  undo of 10,000 walls 2.7 s to 0.12 s.
- A commit that removes entities no longer scans the model once it has the
  index.
- A base GlobalId held by several `IfcRoot`s is refused while any of them
  remains in the model; the check used to see one holder chosen by hash
  order.

**Negative / costs**

- Memory: roughly 40 bytes per entity once built (5 MB at 130,000 entities,
  50 MB at 1.3 million), freed by `Model::drop_derived_indexes`.
- The first call pays the build: 57 ms at 130,000 entities, 1.1 s at 1.3
  million.
- Every write pays a footprint, but only on a model whose indexes are built.
- The undo of a batch scales a little worse than linearly at 1.3 million
  entities (0.85 to 1.3 microseconds a step), which is memory traffic, not a
  term in the batch size.

**Follow-ups / risks to watch**

- The GlobalId index relies on the IFC convention that the GlobalId is the
  first attribute; an entity outside it is a harmless extra candidate.
- A write path added to `Model` must report its footprint, or the indexes go
  stale; `tests/maintained_index.rs` and the two property tests compare the
  maintained indexes with a fresh build after every step.

## Relation to existing code

- `crates/ifc-model/src/index/maintained.rs`, the hooks in `model.rs`,
  `mutation/edit.rs`, `mutation/step.rs` and `mutation/transaction.rs`.
- `crates/openbim-ifc/src/authoring/overlay.rs`, `plan.rs`, `relate.rs`,
  `propagate.rs`, `check.rs`.
