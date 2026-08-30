# ifc-systems implementation plan

Status: partial; SYS-ROOT implemented, connectivity projections remain.
Last updated: 2026-08-30

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

- [x] `SYS-ROOT` - implement systems/distribution systems
  - Evidence: `cargo test -p ifc-systems` (8 tests), 5/5 mutation probes, crate clippy clean.
- [x] `SYS-PORT` - implement port definitions and attachment
  - Evidence: focused view/query tests, invalid/cycle cases, and crate clippy.
- [x] `SYS-CONN` - implement semantic connection graph with cycle budgets
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

- `SYS-ROOT` - `cargo test -p ifc-systems` 8 passed; 5/5 mutation probes -
  Systems are discovered by SCHEMA ANCESTRY, not `Model::ids_of_type`, which
  is an exact-type index and would find no system in a file whose systems are
  all `IfcDistributionSystem`. `IfcZone` is deliberately included: in IFC4 its
  chain is IfcZone -> IfcSystem -> IfcGroup, so excluding it would drop part
  of the model. `IfcRelAssignsToGroup.RelatingGroup` is slot 6, not 5, because
  `RelatedObjectsType` sits at 5. Anomalies are reported, not raised: a file
  with one broken relationship still has a usable system graph.
SYS-PORT - cargo test -p ifc-systems (17 passing) - ports resolve through BOTH
IfcRelNests (IFC4) and IfcRelConnectsPortToElement (IFC2x3, still exported).
IfcPort is abstract so selection is by schema ancestry, never exact type. A
port claimed by both forms keeps the IFC4 one and reports PortAttachedTwice,
because IfcPort.ContainedIn is SET [0:1] and cannot hold two.
SYS-CONN - cargo test -p ifc-systems (17 passing) - ConnectionGraph is
UNDIRECTED: RelatingPort/RelatedPort record authoring order, not flow.
Connections alone leave a physical chain as isolated pairs, since no
relationship joins an element's own inlet to its outlet; NetworkGraph adds
those through-element edges. Both traversals are cycle-safe because ring mains
are normal, not corrupt.
