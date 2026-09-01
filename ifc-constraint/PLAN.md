# ifc-constraint implementation plan

Status: bounded IFC4 constraint domain implemented and locally release-gated.
Last updated: 2026-09-01

This is task state, not ambient context. Follow `AGENTS.md`; claim one task ID,
record blockers/decisions under it, and check it off only with evidence.

## Bounded contract

Exact IFC4 ADD2 TC1 projections and authoring for concrete constraints, resource
relationships, and definition associations. Metric values remain lossless
authored selections; this crate does not evaluate compliance.

## Planned file map

- `src/types.rs`: typed enums and metric value forms
- `src/view.rs`: shared strict decoders and `ConstraintView`
- `src/projection.rs`: metric/objective/relationship projections and queries
- `src/authoring.rs`: typed drafts and pre-staging validation
- `src/error.rs`: typed semantic and authoring failures
- `tests/constraint.rs`: layout, projection, malformed, and atomicity proof

## Work queue

- [x] `CONSTRAINT-CORE` - strict inherited fields and WHERE rules
- [x] `CONSTRAINT-METRIC` - typed benchmark and lossless metric-value projection
- [x] `CONSTRAINT-OBJECTIVE` - benchmark list/aggregator/qualifier projection
- [x] `CONSTRAINT-REL` - resource/definition associations with SELECT validation
- [x] `CONSTRAINT-MUT` - typed transaction-staged authoring
- [ ] `CONSTRAINT-PROOF` - facade/docs, immutable review, full gate, and release

## Completion log

- `CONSTRAINT-CORE/METRIC/OBJECTIVE/REL/MUT` - 5 focused public tests pass exact
  layouts, typed values, WHERE/SELECT failures, staged authoring, direct queries,
  and no-stage rejection; strict all-target Clippy and rustdoc pass.
- Mutation proof - constraint/schema guards participate in a clean-baseline 13/13
  killed resource-domain mutation set, including unknown declaration self-equality;
  compile failures are not counted.
- Facade proof - classification/approval/constraint IDs survive one transaction
  and STEP write/read in `openbim-ifc/tests/resource_domains.rs`.
- Local release proof - fresh isolated `scripts/gate.sh`, VitePress production
  build, generated-doc sync, leakage, and progressive-context gates pass.
