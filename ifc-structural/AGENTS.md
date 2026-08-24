# ifc-structural instructions

Purpose: Borrowed structural analysis model, members, connections, conditions, loads, actions, and result projections.

Follow `../AGENTS.md`. Read `PLAN.md` only for assigned implementation or
roadmap work; keep progress, blockers, and evidence there.

## Boundary

Allowed production dependencies: ifc-model and schema metadata only; no geometry crate.

## Module ownership

- `model.rs`: analysis models/load/result groups
- `member.rs`: curve/surface/varying members
- `connection.rs`: point/curve/surface connections
- `condition.rs`: boundary/connection conditions
- `load.rs`: load value families
- `action.rs`: structural activities/actions
- `result.rs`: reactions/results
- `boundary.rs`: references to structural geometry/topology IDs
- `query.rs`: bounded model graph traversal
- `error.rs`: inconsistent structural semantics

## Invariants

- This crate references geometry/profile entities by EntityId; it does not evaluate shape or link axiolid crates.
- Authored section properties and computed section properties are distinguished.
- Solvers, FEM meshes, and numerical analysis are application/adapter capabilities, not IFC views.

Keep entity views, relationship traversal, mutation, and domain algorithms in
separate files. New child modules remain crate-private until a real public
contract is ready for deliberate re-export.

## Verification

Run targeted tests/clippy, then the package architecture/context gates. Add
fixtures and cycle/invalid-input cases for every relationship traversal.
