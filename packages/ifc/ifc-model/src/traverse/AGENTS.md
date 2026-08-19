# ifc-model traverse instructions

Scope: generic bounded graph traversal primitives. Follow the crate `../../AGENTS.md`. Read `PLAN.md` only for
assigned task(s) `MODEL-TRV` and keep implementation state there.

## Owns

- depth/node/work budgets
- cycle/path diagnostics
- deterministic DFS/BFS helpers

## Does not own

- spatial/product semantics
- unbounded recursion
- global traversal caches

## Growth map

`budget.rs`, `dfs.rs`, `bfs.rs`, `cycle.rs`. These source owners already compile as private scaffold modules. Replace a module's planned-owner marker with its first real contract and tests; do not add parallel placeholders. Every graph operation has deterministic order and explicit limits.
