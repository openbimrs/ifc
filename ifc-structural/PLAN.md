# ifc-structural implementation plan

Status: bounded structural-analysis vertical slice implemented; deferred capability seams remain explicit.
Last updated: 2026-08-31

This is task state, not ambient context. Follow `AGENTS.md`; claim one task ID,
record blockers/decisions under it, and check it off only with evidence.

## Established boundary

`ifc-structural` interprets IFC structural-analysis models, analytical members and
connections, actions, loads, and result-group metadata. It references physical
elements and geometry by `EntityId`; it does not evaluate geometry or solve
mechanics.

`ifc-resource` is a separate construction-management domain for labour,
equipment, materials/products used as capacity or consumables, usage, time,
cost, and process allocation. An analytical member is not an
`IfcConstructionResource` merely because construction consumes resources.

## Planned file map

These paths are compiled private scaffold modules. Implement inside the named
owner and expose a public symbol only through an intentional parent re-export.
Implemented behavior may live in a parent owner while the named child remains a
preserved capability seam for deferred specialization.

- `src/model/analysis.rs`: implemented analysis-model projection
- `src/model/load_group.rs`: implemented load-case/group projection
- `src/model/result_group.rs`: implemented result-group metadata projection
- `src/member/curve.rs`: curve-member specialization seam; bounded behavior implemented in parent
- `src/member/surface.rs`: surface-member specialization seam; bounded behavior implemented in parent
- `src/member/varying.rs`: deferred varying-member specialization
- `src/connection/point.rs`: point-connection specialization seam; bounded behavior implemented in parent
- `src/connection/curve.rs`: curve-connection specialization seam; bounded behavior implemented in parent
- `src/connection/surface.rs`: surface-connection specialization seam; bounded behavior implemented in parent
- `src/condition/translation.rs`: deferred translational boundary conditions
- `src/condition/rotation.rs`: deferred rotational boundary conditions
- `src/load/static.rs`: static-load specialization seam; bounded behavior implemented in parent
- `src/load/dynamic.rs`: deferred dynamic-load specialization
- `src/action/point.rs`: point-action specialization seam; bounded behavior implemented in parent
- `src/action/linear.rs`: curve/linear-action specialization seam; bounded behavior implemented in parent
- `src/action/planar.rs`: surface/planar-action specialization seam; bounded behavior implemented in parent
- `src/result/reaction.rs`: deferred reactions/results

## Implemented surface

- exact canonical IFC2X3, IFC4, or IFC4X3 selection from one file-header token;
- borrowed analysis/load/result group, member, connection, action, and core static-load projections;
- strict missing, dangling, wrong-type, select, and aggregate-cardinality errors;
- version drift for shared placement, axes, action fields, OwnerHistory, and temperature slots;
- deterministic group, member-connection, and activity-assignment queries;
- transaction-staged analysis-model and four static-load authoring paths;
- STEP write/read round-trip coverage.

## Work queue

- [x] `STRUCT-MODEL` - bounded analysis/load/result group projections.
- [x] `STRUCT-MEMBER` - curve/surface identity, type, axis, and thickness projections.
- [x] `STRUCT-CONN` - point/curve/surface connection references and traversal.
- [x] `STRUCT-LOAD` - single, linear, planar, and temperature static loads plus authoring.
- [x] `STRUCT-ACT` - point/curve/surface actions and activity-assignment traversal.
- [ ] `STRUCT-COND` - typed boundary and connection-condition values.
- [ ] `STRUCT-VARYING` - varying member/connection-condition families.
- [ ] `STRUCT-DYNAMIC` - dynamic loads and configurations.
- [ ] `STRUCT-RESULT` - reactions/results beyond result-group metadata.
- [ ] `STRUCT-AUTHOR` - member, connection, action, and relationship authoring.
- [ ] `STRUCT-CROSS` - external physical-product/geometry composition proof.

## Explicit non-capabilities

No solver, FEM mesh, stiffness assembly, geometry evaluation, section-property
calculation, load-combination evaluation, computed reactions/results, arbitrary
EXPRESS `WHERE` evaluation, or general `INVERSE` engine is implemented.

## Completion log

- `STRUCT-MODEL` through `STRUCT-ACT` - 43 package tests pass across IFC2X3,
  IFC4, and IFC4X3; strict clippy and rustdoc pass; 31 semantic mutants killed.
- Full repository and documentation gate evidence belongs to the immutable
  release-candidate review, not to this standing plan.

Append concise entries as `TASK-ID - proof command/result - material decision`.
Do not paste long logs or duplicate standing rules from `AGENTS.md`.
