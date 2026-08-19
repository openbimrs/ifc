# geom-backend-cpu instructions

Purpose: Portable/runtime-specialized CPU execution context.

Allowed internal dependencies: geom-kernel plus L2 algorithms. Follow parent `../AGENTS.md`. Do not read
`PLAN.md` unless assigned implementation or roadmap work.

## Module ownership

features.rs; config.rs; execution.rs. Split a module before unrelated data, validation, and algorithms grow
together. Add no empty placeholder files.

## Invariants

Portable path is oracle. SIMD requires runtime detection. Optional Rayon uses a local bounded pool. Operation providers compose this context and implement a capability trait only when the algorithm works. Never compile the whole workspace for the build host only.

Public values derive `Debug` and `Clone`; add other standard traits only when
semantically valid. Validate unsupported and unavailable paths in tests.
