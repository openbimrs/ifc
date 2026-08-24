# ifc-systems instructions

Purpose: Borrowed building/distribution system, port, flow, zone, and semantic-connectivity projections.

Follow `../AGENTS.md`. Read `PLAN.md` only for assigned implementation or
roadmap work; keep progress, blockers, and evidence there.

## Boundary

Allowed production dependencies: ifc-model and schema metadata only; no geometry/spatial algorithm crate.

## Module ownership

- `system.rs`: systems and distribution systems
- `port.rs`: ports and product nesting
- `connectivity.rs`: semantic port/element connections
- `flow.rs`: flow direction and role semantics
- `zone.rs`: zones/spatial groups
- `assignment.rs`: product/service/system links
- `query.rs`: bounded semantic graph traversal
- `error.rs`: malformed/cyclic system graphs

## Invariants

- System connectivity comes from IFC relationships, not geometric proximity.
- No pressure-flow solver, clash test, routing algorithm, or geometry import enters this crate.
- Direction conflicts and cycles are reported; traversal always has explicit budgets.

Keep entity views, relationship traversal, mutation, and domain algorithms in
separate files. New child modules remain crate-private until a real public
contract is ready for deliberate re-export.

## Verification

Run targeted tests/clippy, then the package architecture/context gates. Add
fixtures and cycle/invalid-input cases for every relationship traversal.
