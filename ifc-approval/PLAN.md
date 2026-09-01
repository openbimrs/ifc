# ifc-approval implementation plan

Status: bounded IFC4 approval domain implemented and locally release-gated.
Last updated: 2026-09-01

This is task state, not ambient context. Follow `AGENTS.md`; claim one task ID,
record blockers/decisions under it, and check it off only with evidence.

## Bounded contract

Exact IFC4 ADD2 TC1 projections and authoring for ApprovalResource's three
entities plus `IfcRelAssociatesApproval`. This does not imply workflow,
authorization, signatures, policy evaluation, or generic EXPRESS execution.

## Planned file map

- `src/view.rs`: shared strict decoders and `ApprovalView`
- `src/projection.rs`: approval/relationship projections and direct queries
- `src/authoring.rs`: typed drafts and pre-staging validation
- `src/error.rs`: typed semantic and authoring failures
- `tests/approval.rs`: layout, projection, malformed, and atomicity proof

## Work queue

- [x] `APPROVAL-CORE` - strict `IfcApproval` projection and WR identifier/name refusal
- [x] `APPROVAL-REL` - approval-to-approval, resource, and definition associations
- [x] `APPROVAL-QUERY` - deterministic direct lookup by approval/resource/definition
- [x] `APPROVAL-MUT` - typed transaction-staged authoring with projected references
- [ ] `APPROVAL-PROOF` - facade/docs, immutable review, full gate, and release

## Completion log

- `APPROVAL-CORE/REL/QUERY/MUT` - 4 focused public tests pass exact layouts,
  WHERE/SELECT/set failures, staged references, direct queries, and no-stage
  rejection; strict all-target Clippy and rustdoc pass.
- Mutation proof - approval/schema guards participate in a clean-baseline 13/13
  killed resource-domain mutation set, including unknown declaration self-equality;
  compile failures are not counted.
- Facade proof - classification/approval/constraint IDs survive one transaction
  and STEP write/read in `openbim-ifc/tests/resource_domains.rs`.
- Local release proof - fresh isolated `scripts/gate.sh`, VitePress production
  build, generated-doc sync, leakage, and progressive-context gates pass.
