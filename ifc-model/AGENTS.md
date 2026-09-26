# ifc-model instructions

Purpose: Schema-agnostic entity graph and stable ports used by every IFC adapter and projection.

Follow `../AGENTS.md`. Read `PLAN.md` only when assigned implementation or
roadmap work; record progress and blockers there, not here.

## Boundary

Allowed production dependencies: no internal IFC or geometry crate.

## Module ownership

- `codec.rs`: serialization port only
- `id.rs/value.rs`: lossless generic values and typed handles
- `model.rs`: record ownership and basic lookup
- `lazy.rs`: slots decoded on first access and the `EntitySource` port a
  codec implements (ADR 0015); the model never knows the source format
- `index.rs`: derived indexes, never domain semantics
- `relation.rs/traverse.rs/spatial.rs`: generic graph queries with explicit budgets
- `guid.rs`: IFC compressed GUID value codec
- `authored_dump.rs`: coverage-audit hook, `authored-dump` feature only

## Invariants

- Unknown entity and value forms survive codec round trips.
- No schema entity names, material concepts, or geometry types enter this crate.
- Indexes are derived state and must remain coherent across mutation.

Keep `lib.rs` delegating, keep child modules crate-private until they own a real
public contract, and split view/data, traversal, mutation, and validation before
they grow together.

## Coverage audit

`authored_dump.rs` answers which entity types a test run actually
builds. Static scanning of writer call sites cannot: a type name
reaches `Entity::new` through a const, a catalogue row, or a match
arm returning it into a tuple.

```
AUTHORED_DUMP=/tmp/dump \
  cargo test --workspace --all-features --features ifc-model/authored-dump
python3 ../scripts/authored-coverage.py /tmp/dump
```

Three origins are recorded. Only `create` proves a writer exists:

- `create`: the authoring path, `Transaction::create`.
- `insert`: `Model::insert`, the chokepoint every entity lands
  through -- committed transactions, `push`, and codec loads. An
  entity seen only here exists in files but no writer can make one.
- `retype`: renames in place, so it can name a type no writer built.

Hooking `create` and `push` alone missed two paths:
`Transaction::stage(Edit::Create { .. })` and `retype`.

The feature is off by default and inert without the variable, which
`tests/authored_dump_inert.rs` holds in place: the gate builds
`--all-features`, so this ships compiled into every gate run.
## Verification

Run targeted crate tests and clippy first, then the package architecture/context
gates from `../AGENTS.md`. Record exact exit evidence in `PLAN.md`.
