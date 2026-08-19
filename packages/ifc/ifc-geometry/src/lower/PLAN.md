# ifc-geometry lower plan

Status: active scaffold under parent tasks `GEOM-CONTRACT`, `GEOM-SESSION`,
`GEOM-CTX`, `GEOM-PLACE`, `GEOM-PROFILE`, `GEOM-CURVE`, `GEOM-SURFACE`,
`GEOM-BREP`, `GEOM-SOLID`, and `GEOM-MAP`.
Last updated: 2026-08-19

Follow `AGENTS.md`. Claim one local task, leave blockers/decisions beneath it,
and check it off only after the proof runs.

## Work queue

- [ ] `LOW-CONTRACT` - validate/normalize every source direction and axis exactly once
  - Implements: `GEOM-CONTRACT`.
  - Proof: non-unit/zero-vector contract tests against `geom-model` semantics.
- [x] `LOW-SESSION` - shared builder, EntityId memo, active stack, roots, and provenance
  - Implements: `GEOM-SESSION`.
  - Proof: `cargo test -p ifc-geometry` (413 passing); `tests/lower_session.rs`
    covers cross-family combination, entity and shared-profile memoization,
    frame-distinct keys, cycle detection, depth budget, and graph-fault
    attribution.
  - Decision: `LOW-CONTRACT` was NOT a real prerequisite; direction validation
    already lives in `resource::direction` and the session is agnostic to it.
  - Note: provenance remains open under `LOW-PROV`; the session carries the
    memo and active stack only.
- [x] `LOW-DISPATCH` - total entity dispatcher and typed unsupported results
  - Proof: `tests/lower_dispatch_corpus.rs` walks the committed corpus; every
    representation item either lowers or returns a typed `Unsupported` naming a
    real entity. Census: 25 lowered; unsupported by family: FACETEDBREP 20,
    MAPPEDITEM 24, SWEPTDISKSOLID 3, CSGSOLID 1, HALFSPACESOLID 1,
    BOOLEANRESULT 1, BOOLEANCLIPPINGRESULT 1.
  - Decision: a nested failure reports the INNERMOST unlowerable entity, not
    the outer item that referenced it, so the report points at the actual gap.
  - Implemented families: EXTRUDEDAREASOLID, REVOLVEDAREASOLID, BOOLEANRESULT,
    BOOLEANCLIPPINGRESULT. Planned families are declared as data in
    `dispatch::PLANNED`, each with a concrete stated reason.
- [ ] `LOW-CONTEXT` - units/context/placement composition exactly once
  - Requires: `LOW-CONTRACT`, `LOW-SESSION`, `INPUT-REP`, `INPUT-PRODUCT`.
  - Implements: `GEOM-CTX`, `GEOM-PLACE`.
  - Proof: focused tests, crate clippy, and relevant declaration/corpus gate.
- [ ] `LOW-EXACT` - exact profile/curve/surface node construction
  - Requires: `LOW-CONTRACT`, `LOW-SESSION`, `INPUT-PROFILE`, `INPUT-MAT`.
  - Implements: `GEOM-PROFILE`, `GEOM-CURVE`, `GEOM-SURFACE`.
  - Proof: focused tests, crate clippy, and relevant declaration/corpus gate.
- [ ] `LOW-BREP` - topology plus geometry handles
  - Requires: `LOW-DISPATCH`, `LOW-EXACT`, `INPUT-TOPO`.
  - Implements: `GEOM-BREP`.
  - Proof: focused tests, crate clippy, and relevant declaration/corpus gate.
- [ ] `LOW-TESS` - preserve authored n-gons/holes/triangles without retessellation
  - Requires: `LOW-DISPATCH`, `INPUT-TOPO`.
  - Implements: `GEOM-TESS`.
  - Proof: focused tests, crate clippy, and relevant declaration/corpus gate.
- [ ] `LOW-MAP` - Instance nodes with cycle/depth budgets
  - Requires: `LOW-DISPATCH`, `LOW-CONTEXT`.
  - Implements: `GEOM-MAP`.
  - Proof: focused tests, crate clippy, and relevant declaration/corpus gate.
- [ ] `LOW-PROV` - separate NodeId-to-IFC provenance map
  - Requires: `LOW-SESSION`.
  - Proof: focused tests, crate clippy, and relevant declaration/corpus gate.
- [ ] `LOW-CENSUS` - lower every supported corpus item and classify every unsupported item
  - Requires: `LOW-DISPATCH`.
  - Implements: `GEOM-CENSUS`.
  - Proof: focused tests, crate clippy, and relevant declaration/corpus gate.

## Completion log

Append `TASK-ID - proof - material decision`; keep long logs out of this file.

- `LOW-SESSION` - 413 tests pass; 4/4 mutation probes caught (cycle, depth,
  dispatch reason, profile memo) - family lowerers now append into one caller
  owned builder and return `NodeId`; `finish` is the only freeze point.
- `LOW-DISPATCH` - corpus census above - unimplemented families are declared
  data with stated reasons rather than a wildcard no-op.
