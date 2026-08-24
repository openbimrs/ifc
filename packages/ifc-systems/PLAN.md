# ifc-systems implementation plan

Status: architecture scaffold; systems and connectivity projections remain to implement.
Last updated: 2026-08-19

This is task state, not ambient context. Follow `AGENTS.md`; claim one task ID,
record blockers/decisions under it, and check it off only with evidence.

## Established boundary

Borrowed building/distribution system, port, flow, zone, and semantic-connectivity projections.

## Planned file map

These paths are compiled private scaffold modules. Implement inside the named
owner and expose a public symbol only through an intentional parent re-export.

- `src/system/group.rs`: IfcSystem and group semantics
- `src/system/distribution.rs`: distribution systems
- `src/port/definition.rs`: IfcPort/DistributionPort
- `src/port/assignment.rs`: port nesting/attachment
- `src/connectivity/relation.rs`: port/element connections
- `src/connectivity/graph.rs`: semantic graph
- `src/connectivity/traversal.rs`: bounded traversal
- `src/flow/direction.rs`: flow direction/select semantics
- `src/flow/role.rs`: source/sink role
- `src/zone/definition.rs`: zones
- `src/zone/spatial_group.rs`: spatial zone/group links
- `src/assignment/service.rs`: services-building relationships

## Work queue

- [ ] `SYS-ROOT` - implement systems/distribution systems
  - Evidence: focused view/query tests, invalid/cycle cases, and crate clippy.
- [ ] `SYS-PORT` - implement port definitions and attachment
  - Evidence: focused view/query tests, invalid/cycle cases, and crate clippy.
- [ ] `SYS-CONN` - implement semantic connection graph with cycle budgets
  - Evidence: focused view/query tests, invalid/cycle cases, and crate clippy.
- [ ] `SYS-FLOW` - implement direction/role consistency checks
  - Evidence: focused view/query tests, invalid/cycle cases, and crate clippy.
- [ ] `SYS-ZONE` - implement zones and spatial groups
  - Evidence: focused view/query tests, invalid/cycle cases, and crate clippy.
- [ ] `SYS-QUERY` - deterministic upstream/downstream queries without geometry
  - Evidence: focused view/query tests, invalid/cycle cases, and crate clippy.

## Completion log

Append concise entries as `TASK-ID - proof command/result - material decision`.
Do not paste long logs or duplicate standing rules from `AGENTS.md`.
