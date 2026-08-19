# geom-kernel instructions

Purpose: Backend-neutral capability contract.

Allowed internal dependencies: geom-core, geom-model, geom-mesh. Follow parent `../AGENTS.md`. Do not read
`PLAN.md` unless assigned implementation or roadmap work.

## Module ownership

backend.rs; capability.rs; execution.rs; error.rs; compile.rs; boolean.rs. Split a module before unrelated data, validation, and algorithms grow
together. Add no empty placeholder files.

## Invariants

No concrete implementation or hardware API dependency. Open traits remain downstream-implementable. A trait implementation is the capability proof; descriptors carry identity only. Registries store executable trait objects, never boolean capability claims.

Public values derive `Debug` and `Clone`; add other standard traits only when
semantically valid. Validate unsupported and unavailable paths in tests.
