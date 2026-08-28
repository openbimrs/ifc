# 0010 -- Checked mutation is a model-level primitive, not a bare accessor

- Status: Accepted
- Date: 2026-08-28
- Deciders: openbimrs contributors
- Supersedes: --

## Context

`Model` shipped with `insert`/`push` for construction and no mutation path at all. An editor moving a placed point, renaming an entity, or restyling an item had exactly one option: clone the entity, mutate the clone, `insert` it back over the same id. That path works but bypasses every invariant `ifc-author`'s `EntityBuilder::insert` enforces on construction -- reference resolution, declared-type checks, GlobalId uniqueness -- because it is a raw `Model::insert` call, indistinguishable from a codec replaying a file it just parsed.

The naive fix, `Model::get_mut(&mut self, id) -> Option<&mut Entity>`, reopens the exact problem `Model::insert` already solves for id reuse: `Entity::type_name` is also the key into `Model::by_type`. A caller that mutates `type_name` through a bare `&mut Entity` desynchronizes the index silently -- `ids_of_type` then reports the old type, the new type, or both, depending on timing.

`ifc-author` (ADR 0007) owns schema-checked *construction*: arity, unknown attributes, required-but-missing, declared-type mismatches. It does not yet own schema-checked *editing* of an entity already in the model -- that is future work, not blocked by this decision.

## Decision

`ifc-model` gains four checked, schema-agnostic edit operations as inherent `Model` methods, implemented in `mutation::edit` (previously a scaffold placeholder) rather than as a bare accessor:

```rust
impl Model {
    pub fn set_attribute(&mut self, id: EntityId, index: usize, value: Value) -> Option<Value>;
    pub fn set_attributes(&mut self, id: EntityId, edits: impl IntoIterator<Item = (usize, Value)>) -> Option<Vec<Value>>;
    pub fn retype(&mut self, id: EntityId, type_name: impl Into<Arc<str>>) -> Option<Arc<str>>;
    pub fn remove(&mut self, id: EntityId) -> Option<Entity>;
}
```

Every operation that can touch `type_name` (`retype`, `remove`) reindexes `by_type` itself, the same logic `Model::insert` already runs for id reuse. `set_attribute`/`set_attributes` never touch `type_name`, so they need no reindex. No bare `&mut Entity` or `&mut HashMap` is ever exposed publicly; the model keeps sole custody of its derived index.

These operations are schema-agnostic on purpose, matching `Model`'s existing invariant scope: they trust the caller with slot indices and do not know an entity's declared attribute count. They are the primitive a future schema-checked `EntityUpdate` (mirroring `EntityBuilder`) would build on, not a replacement for one.

## Alternatives considered

| Option | Why not |
| --- | --- |
| Bare `get_mut(&mut self, id) -> Option<&mut Entity>` | Lets a caller mutate `type_name` without touching `by_type`, silently desynchronizing `ids_of_type` -- the exact bug class `Model::insert` already guards against for id reuse |
| Schema-checked `EntityUpdate` builder now (mirroring `EntityBuilder::insert`) | Real improvement, but a separate, larger `ifc-author` design task; blocking the model-level primitive on it leaves editors with no safe path at all in the meantime |
| Full transactional preflight/commit (`mutation::transaction`, `mutation::conflict`) | Different problem -- atomic multi-entity edits and conflict diagnostics -- tracked separately in `mutation/PLAN.md` under `MUT-PREFLIGHT`/`MUT-COMMIT`; not needed for a single-entity edit |

## Consequences

**Positive**

- Editors (open-signs' "move a placed sign", rename, restyle) have a documented, index-safe mutation path instead of an unsafe clone-mutate-insert workaround.
- The by-type index cannot desync through the public API: every path that changes `type_name` reindexes it.
- `mutation::edit` graduates from scaffold placeholder to owned module per ADR 0005, without touching `mutation::transaction` or `mutation::conflict`, which remain planned.

**Negative / costs**

- No reference-integrity, arity, or declared-type checking on edits -- `remove` can leave dangling references (by design: detectable via `dangling_references`, not auto-repaired), and `set_attribute` will happily write a value of the wrong shape into a slot. A schema-checked `EntityUpdate` is future work, tracked in `ifc-model/src/mutation/PLAN.md`.

**Follow-ups / risks to watch**

- `MUT-PREFLIGHT` / `MUT-COMMIT` (ID/reference/index conflicts, atomic multi-edit commit) remain open in `mutation/PLAN.md`.
- `ifc-author` could grow a schema-checked `EntityUpdate` on top of these primitives, extending "authored edits are checked" from creation to modification -- not started here.

## Relation to existing code

- `ifc-model/src/mutation/edit.rs` -- the four operations.
- `ifc-model/src/model.rs` -- `entities_mut`/`by_type_mut`/`order_mut`, crate-private seams `mutation::edit` uses; not part of the public API.
- `ifc-model/tests/mutation_edit.rs` -- in-memory contract tests (index consistency, no-op cases, missing-id cases).
- `openbim-ifc/tests/mutation_roundtrip.rs` -- STEP write/read round trip proving an edit survives serialization.
- Fixes https://github.com/openbimrs/ifc/issues/3.

