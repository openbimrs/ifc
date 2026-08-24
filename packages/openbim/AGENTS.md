# openbim instructions

Purpose: Facade over the openBIM standards; each feature re-exports one crate.

Follow `../AGENTS.md`. Read `PLAN.md` only for assigned implementation or
roadmap work; keep progress, blockers, and evidence there.

## Boundary

Re-exports only. Never define a type here -- consumers may also depend on the leaf crate directly.

## Status

Facade wired; leaves reserved.
