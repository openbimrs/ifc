# ifc-structural implementation plan

Status: architecture scaffold; structural resource projections remain to implement.
Last updated: 2026-08-19

This is task state, not ambient context. Follow `AGENTS.md`; claim one task ID,
record blockers/decisions under it, and check it off only with evidence.

## Established boundary

Borrowed structural analysis model, members, connections, conditions, loads, actions, and result projections.

## Planned file map

These paths are compiled private scaffold modules. Implement inside the named
owner and expose a public symbol only through an intentional parent re-export.

- `src/model/analysis.rs`: IfcStructuralAnalysisModel
- `src/model/load_group.rs`: load cases/groups
- `src/model/result_group.rs`: result groups
- `src/member/curve.rs`: curve members
- `src/member/surface.rs`: surface members
- `src/member/varying.rs`: varying members
- `src/connection/point.rs`: point connections
- `src/connection/curve.rs`: curve connections
- `src/connection/surface.rs`: surface connections
- `src/condition/translation.rs`: translational conditions
- `src/condition/rotation.rs`: rotational conditions
- `src/load/static.rs`: static load values
- `src/load/dynamic.rs`: dynamic load values
- `src/action/point.rs`: point actions
- `src/action/linear.rs`: linear actions
- `src/action/planar.rs`: planar actions
- `src/result/reaction.rs`: reactions/results

## Work queue

- [ ] `STRUCT-MODEL` - implement analysis/load/result group views
  - Evidence: focused view/query tests, invalid/cycle cases, and crate clippy.
- [ ] `STRUCT-MEMBER` - implement all member families and geometry references
  - Evidence: focused view/query tests, invalid/cycle cases, and crate clippy.
- [ ] `STRUCT-CONN` - implement connections and boundary conditions
  - Evidence: focused view/query tests, invalid/cycle cases, and crate clippy.
- [ ] `STRUCT-LOAD` - implement load value/select families with units
  - Evidence: focused view/query tests, invalid/cycle cases, and crate clippy.
- [ ] `STRUCT-ACT` - implement action/activity relationships
  - Evidence: focused view/query tests, invalid/cycle cases, and crate clippy.
- [ ] `STRUCT-RESULT` - implement result/reaction projections
  - Evidence: focused view/query tests, invalid/cycle cases, and crate clippy.
- [ ] `STRUCT-CROSS` - prove authored profile references compose externally with geometry without dependency
  - Evidence: focused view/query tests, invalid/cycle cases, and crate clippy.

## Completion log

Append concise entries as `TASK-ID - proof command/result - material decision`.
Do not paste long logs or duplicate standing rules from `AGENTS.md`.
