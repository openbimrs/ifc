# 0022 — Undo records exact inverses in the model

- **Status:** Accepted
- **Date:** 2026-10-27
- **Deciders:** openbimrs contributors
- **Supersedes:** —

## Context

`Transaction::commit` is atomic, but `Applied` returns only the created ids
and the removed entities, never the prior slot values, so a caller cannot
undo a commit ([#465](https://github.com/openbimrs/ifc/issues/465)). The
requirement is stronger than "get the content back": the restored model must
write the same STEP bytes, which depends on file order, type-index order,
attribute counts, the header and the next free id. Authoring also derives
`GlobalId`s from the batch seed and ids
([#548](https://github.com/openbimrs/ifc/issues/548)), so replaying a batch
after an undo must allocate the same ids.

## Decision

`ifc-model` records the exact inverse of every primitive write, and undo and
redo are built on that.

- While a recording is open (`Model::begin_recording`), each primitive write
  (`insert`, `insert_lazy`, `push`, `set_attribute`, `set_attributes`,
  `retype`, `remove`, `header_mut`) keeps the write that undoes it, including
  the position in file order and in the type bucket.
- `Inverse` owns those steps. Applying it restores the model and returns the
  redo `Inverse`.
- `History` is a bounded undo and redo stack of `Inverse`s. `History::track`
  wraps any change (an authoring batch, a property edit) as one entry.
- `Transaction::commit_reversible` is `commit` plus the recording, and
  `Transaction::set_header` stages a header replacement.

## Alternatives considered

| Option | Why not |
| --- | --- |
| An inverse transaction built in `Transaction::commit` | A `Create` appends, so file order, bucket order and the next id cannot be restored; direct primitives and `header_mut` are missed; every facade outcome would need a new field, and `AuthoringOutcome` derives `Eq` which `Value` cannot satisfy. |
| A whole-model snapshot per entry | Memory proportional to the model for every edit. |
| History in the facade | Splits the revision invariant across crates for no gain. |
| A new `Edit` variant for the header | `Edit` is matched exhaustively downstream; a variant would break those crates. |

## Consequences

**Positive**

- Undo restores the exact bytes, including for lazily loaded models.
- No facade signature changes: any change that takes `&mut Model` is
  trackable.
- A seed is free again after its batch is undone, so a replay is accepted
  and reproduces the same ids and `GlobalId`s.

**Negative / costs**

- The revision is never restored. An undo is a new change and bumps it, or a
  transaction opened before the undo could commit against different state.
  An `Inverse` therefore applies only at the revision it was recorded up to.
- Revisions are process-unique: every change draws a new value from one
  process-wide counter. An inverse or history recorded on one model is
  refused by another (`StaleRevision`), where per-model counters would have
  let it delete an unrelated entity or panic. A clone taken at revision R
  accepts an inverse recorded up to R until either side changes. Revisions
  are no longer consecutive; nothing relied on that.
- Ids are reused after undoing a create (the next free id is restored). An
  outside reference to an undone id may later name a different entity.
- `Model::header_mut` now bumps the revision.
- The history bound counts entries, not bytes: removed entities stay in
  memory until their entry is dropped.
- A change made outside the history makes the history stale; `undo` and
  `redo` refuse with `StaleRevision` and the next tracked change clears it.

**Follow-ups / risks to watch**

- The bindings expose the history in a later change.
- Grouping several calls into one entry is `History::track` in Rust only.

## Relation to existing code

- `crates/ifc-model/src/mutation/`: `step.rs`, `record.rs`, `inverse.rs`,
  `history.rs`, and the hooks in `edit.rs` and `model.rs`.
- `crates/openbim-ifc/src/lib.rs` re-exports the public items.
