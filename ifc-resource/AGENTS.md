# ifc-resource instructions

Purpose: bounded IFC4/IFC4X3 construction-resource projections,
actor/inventory metadata, usage quantities, queries, and authoring.

Follow `../AGENTS.md`. Read `PLAN.md` only for assigned implementation or roadmap work; keep progress, blockers, and evidence there.

## Implemented boundary

Public behavior is restricted to IFC4 ADD2 TC1 and IFC4X3 ADD2 (identical entity shapes verified against the normative .exp files):

- schema-resolved borrowed projections for six concrete `IfcConstructionResource` occurrence kinds;
- schema-resolved borrowed projections for six concrete `IfcConstructionResourceType` kinds, with `IfcRelDefinesByType` assignment resolution;
- `IfcPerson`, `IfcOrganization`, `IfcOrganizationRelationship`, `IfcPersonAndOrganization`, and `IfcActorRole` projections, enforcing `IdentifiablePerson`, `ValidSetOfNames`, and `WR1`;
- `IfcInventory` metadata and jurisdiction, with `IfcRelAssignsToGroup` membership resolution;
- authored `IfcResourceTime` scalar metadata;
- `IfcPhysicalSimpleQuantity` (all six concrete measure kinds) and `IfcPhysicalComplexQuantity` usage-quantity projections, enforcing the shared non-negative/finite rule and `NoSelfReference`;
- deterministic `IfcRelAssignsToResource` lookup with explicit `RelatedObjectsType` category matching;
- authored-order `IfcRelNests` resource composition with explicit cycle and budget failures;
- transaction-staged creation of selected resources, usage records, allocations, and nesting relationships;
- pre-staging refusal of duplicate model-wide `GlobalId` values, second resource parents, and cycle creation.

IFC2X3 is an explicit unsupported-schema result: it does not declare `IfcConstructionResourceType`, `IfcResourceTime`, or `PredefinedType` on `IfcConstructionEquipmentResource`/`IfcCrewResource`, so there is no normative behavior to project (verified against `IFC2X3_TC1.exp`; do not add a reduced IFC2X3 path without a fresh design decision). The crate does not schedule, level, calculate cost/quantity formulas, parse calendars, solve logistics, resolve units against a project unit assignment, or expose generic EXPRESS `WHERE`/`INVERSE` execution.

## Boundary

Allowed production dependencies: `ifc-schema` and `ifc-model` only. Scheduling and costing compose at the facade/application layer; do not add sibling domain-crate dependencies.

## Module ownership

- `author/`: schema-checked drafts and transaction-staged creation
- `resource/`: construction-resource occurrences, resource types, and nesting
- `actor/`: person, organization, organization-relationship, and actor-role projections
- `inventory/`: inventory metadata and group-membership projections
- `usage/`: authored resource-time metadata and physical-quantity projections
- `query/`: resource assignment relations
- `view.rs`: shared schema-resolved record decoder
- `error.rs`: typed malformed-graph and authoring failures
- `labour.rs`, `equipment.rs`, `crew.rs`, `material.rs`: private ownership markers; public specialization currently comes through `ResourceKind`

## Invariants

- A construction resource is domain semantics, not a runtime thread/CPU/GPU resource.
- Resource usage values remain authored values; no inferred schedule or cost is manufactured.
- Named schema attributes resolve inherited STEP slots; never hard-code positional indices.
- SELECT membership, aggregate minimums, SET uniqueness, dangling references, and target ancestry fail with typed errors.
- Traversal always takes `ifc_model::Budget`, preserves authored order, and reports cycles rather than silently skipping them.
- Rejected drafts must leave both model length and revision unchanged.
- Physical-quantity and actor projections preserve authored values; they never evaluate formulas, resolve units, or interpret calendars.

## Verification

Run:

```bash
cargo +1.88.0 test -p ifc-resource --all-targets
cargo +1.88.0 clippy -p ifc-resource --all-targets -- -D warnings
```

Then run repository architecture/context checks and the full gate. Add malformed scalar/reference/cardinality cases and mutation evidence for every semantic branch.
