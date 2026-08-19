# ifc-resource instructions

Purpose: Borrowed construction resource, actor, inventory, crew, equipment, labour, and usage projections.

Follow `../AGENTS.md`. Read `PLAN.md` only for assigned implementation or
roadmap work; keep progress, blockers, and evidence there.

## Boundary

Allowed production dependencies: ifc-model and schema metadata only.

## Module ownership

- `actor.rs`: people, organizations, roles, addresses
- `resource.rs`: resource base/type/nesting
- `labour.rs`: labor resources and skills
- `equipment.rs`: construction equipment resources
- `crew.rs`: crews and composition
- `inventory.rs`: inventory and contained assets
- `usage.rs`: time/quantity/cost usage metadata
- `query.rs`: allocation/availability relationships
- `error.rs`: malformed resource graphs

## Invariants

- A construction resource is domain semantics, not a runtime thread/CPU/GPU resource.
- Resource usage quantities remain authored values with explicit units.
- Scheduling and costing compose at application level; this crate does not depend on their feature crates.

Keep entity views, relationship traversal, mutation, and domain algorithms in
separate files. New child modules remain crate-private until a real public
contract is ready for deliberate re-export.

## Verification

Run targeted tests/clippy, then the package architecture/context gates. Add
fixtures and cycle/invalid-input cases for every relationship traversal.
