# ifc-approval instructions

Purpose: bounded IFC4 approval-resource projections, relationships, queries, and authoring.

Follow `../AGENTS.md`. Read `PLAN.md` only for assigned implementation or roadmap work; keep progress, blockers, and evidence there.

## Implemented boundary

- `IfcApproval`, `IfcApprovalRelationship`, and `IfcResourceApprovalRelationship`;
- `IfcRelAssociatesApproval` object associations;
- strict borrowed projections, deterministic direct queries, and transaction-staged authoring.

Approval status, level, and qualifier are authored facts, not authorization,
signatures, workflow, policy evaluation, or automatic propagation.

## Boundary

Allowed production dependencies are `ifc-model`, `ifc-schema`, and shared error
support. Views borrow the model. Authoring stages on a caller-owned transaction;
this crate never commits or performs external I/O.

## Module ownership

- `src/view.rs`: strict slot, aggregate, and SELECT decoding
- `src/projection.rs`: approval and relationship projections/queries
- `src/authoring.rs`: typed drafts and transaction staging
- `src/error.rs`: typed semantic and authoring refusals
- `tests/approval.rs`: public behavior, malformed input, and atomicity proof

## Invariants

- Approval records are not rooted; never invent a `GlobalId` for `IfcApproval`.
- Rooted object associations validate compressed GlobalIds.
- Resource and definition SELECT endpoints resolve through bundled IFC4 metadata.
- Rejected drafts leave transaction length unchanged.
- Output ordering follows deterministic model entity order.

## Verification

Run focused tests, strict all-target Clippy/rustdoc, approval mutations, the facade
STEP join, architecture/context tests, then the full repository gate.
