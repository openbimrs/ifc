# ifc-constraint instructions

Purpose: bounded IFC4 metric, objective, relationship, query, and authoring semantics.

Follow `../AGENTS.md`. Read `PLAN.md` only for assigned implementation or roadmap work; keep progress, blockers, and evidence there.

## Implemented boundary

- concrete `IfcMetric` and `IfcObjective` over inherited `IfcConstraint` slots;
- `IfcResourceConstraintRelationship` and `IfcRelAssociatesConstraint`;
- typed enums, preserved metric-value SELECTs, deterministic direct queries, and transaction-staged authoring.

The crate preserves authored constraints but does not evaluate compliance,
formulas, references, tables, time series, or unit conversions.

## Boundary

Allowed production dependencies are `ifc-model`, `ifc-schema`, and shared error
support. Views borrow the model. Authoring stages on a caller-owned transaction;
this crate never commits or calls sibling domains.

## Module ownership

- `src/types.rs`: typed IFC4 enums and metric value forms
- `src/view.rs`: strict slot, aggregate, and SELECT decoding
- `src/projection.rs`: metric/objective/relationship projections and queries
- `src/authoring.rs`: typed drafts and transaction staging
- `src/error.rs`: typed semantic and authoring refusals
- `tests/constraint.rs`: public behavior, malformed input, and atomicity proof

## Invariants

- Metric values are preserved, never evaluated or normalized.
- User-defined grade/qualifier WHERE rules fail closed.
- Resource, definition, actor, reference, and metric SELECT endpoints use IFC4 metadata.
- Rejected drafts leave transaction length unchanged.
- Objective benchmark LIST order is preserved.

## Verification

Run focused tests, strict all-target Clippy/rustdoc, constraint mutations, the
facade STEP join, architecture/context tests, then the full repository gate.
