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
- [ ] `LOW-SESSION` - shared builder, EntityId memo, active stack, roots, and provenance
  - Requires: `LOW-CONTRACT`.
  - Implements: `GEOM-SESSION`.
  - Proof: shared-profile, boolean-tree, mapped-cycle, and multi-root tests.
- [ ] `LOW-DISPATCH` - total entity dispatcher and typed unsupported results
  - Requires: `LOW-SESSION`.
  - Proof: focused tests, crate clippy, and relevant declaration/corpus gate.
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
