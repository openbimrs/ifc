# ifc-resource instructions

Purpose: bounded IFC4 construction-resource projections, queries, and authoring, with reserved private modules for future actor and inventory work.

Follow `../AGENTS.md`. Read `PLAN.md` only for assigned implementation or roadmap work; keep progress, blockers, and evidence there.

## Implemented boundary

Public behavior is restricted to IFC4 ADD2 TC1:

- schema-resolved borrowed projections for six concrete `IfcConstructionResource` occurrence kinds;
- authored `IfcResourceTime` scalar metadata;
- deterministic `IfcRelAssignsToResource` lookup with explicit `RelatedObjectsType` category matching;
- authored-order `IfcRelNests` resource composition with explicit cycle and budget failures;
- transaction-staged creation of selected resources, usage records, allocations, and nesting relationships;
- pre-staging refusal of duplicate model-wide `GlobalId` values, second resource parents, and cycle creation.

IFC2X3 and IFC4X3 are explicit unsupported-schema results. The crate does not schedule, level, calculate cost/quantity, parse calendars, solve logistics, or expose generic EXPRESS `WHERE`/`INVERSE` execution.

## Boundary

Allowed production dependencies: `ifc-model` and schema metadata only. Scheduling and costing compose at the facade/application layer; do not add sibling domain-crate dependencies.

## Module ownership

- `author/`: schema-checked drafts and transaction-staged creation
- `resource/`: construction-resource occurrences and nesting
- `usage/`: authored resource-time metadata
- `query/`: resource assignment relations
- `view.rs`: shared schema-resolved record decoder
- `error.rs`: typed malformed-graph and authoring failures
- `actor.rs`, `inventory.rs`, and their children: private scaffolds, not capabilities
- `labour.rs`, `equipment.rs`, `crew.rs`, `material.rs`: private ownership markers; public specialization currently comes through `ResourceKind`

## Invariants

- A construction resource is domain semantics, not a runtime thread/CPU/GPU resource.
- Resource usage values remain authored values; no inferred schedule or cost is manufactured.
- Named schema attributes resolve inherited STEP slots; never hard-code positional indices.
- SELECT membership, aggregate minimums, SET uniqueness, dangling references, and target ancestry fail with typed errors.
- Traversal always takes `ifc_model::Budget`, preserves authored order, and reports cycles rather than silently skipping them.
- Rejected drafts must leave both model length and revision unchanged.

## Verification

Run:

```bash
cargo +1.88.0 test -p ifc-resource --all-targets
cargo +1.88.0 clippy -p ifc-resource --all-targets -- -D warnings
```

Then run repository architecture/context checks and the full gate. Add malformed scalar/reference/cardinality cases and mutation evidence for every semantic branch.
