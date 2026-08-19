# geom-backend-gpu instructions

Purpose: API-neutral GPU executor adapter.

Allowed internal dependencies: geom-kernel, geom-model, geom-mesh. Follow parent `../AGENTS.md`. Do not read
`PLAN.md` unless assigned implementation or roadmap work.

## Module ownership

device.rs; executor.rs; adapter.rs. Split a module before unrelated data, validation, and algorithms grow
together. Add no empty placeholder files.

## Invariants

No fake device or claimed operation. Concrete API crates implement
`GpuGraphExecutor` or another narrow executor. Batch boundaries amortize
transfer. The adapter validates device preference, f32/f64 policy, graph-root
ownership, and result cardinality; concrete executors must honor forwarded
determinism and memory-budget requirements.

Public values derive `Debug` and `Clone`; add other standard traits only when
semantically valid. Validate unsupported and unavailable paths in tests.
