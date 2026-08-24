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
  - Proof: non-unit/zero-vector contract tests against `axiolid-model` semantics.
- [x] `LOW-SESSION` - shared builder, EntityId memo, active stack, roots, and provenance
  - Implements: `GEOM-SESSION`.
  - Proof: `cargo test -p ifc-geometry` (413 passing); `tests/lower_session.rs`
    covers cross-family combination, entity and shared-profile memoization,
    frame-distinct keys, cycle detection, depth budget, and graph-fault
    attribution.
  - Decision: `LOW-CONTRACT` was NOT a real prerequisite; direction validation
    already lives in `resource::direction` and the session is agnostic to it.
  - Note: source attribution is implemented separately below.
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
- [x] `LOW-CONTEXT` - units/context/placement composition exactly once
  - Requires: `LOW-SESSION`, `INPUT-REP`, `INPUT-PRODUCT`.
  - Implements: `GEOM-PLACE`.
  - Proof: `tests/lower_product.rs` (4 tests) plus the ifc-cli corpus gate
    `products_are_distributed_by_their_placements`.
  - Decision: the placement chain is composed in FILE units and converted to
    metres exactly once at the end. Converting per link would scale a depth-n
    chain n times; every family lowerer already converts its own local
    placement, so the world frame handed to them must arrive in metres.
  - Decision: representation selection is a preference list (Body, Facetation)
    and never the first entry. Wall #928204 in issue_098_wall_W.ifc lists its
    Axis Curve2D before its Body, so first-wins yields a line, not a solid.
  - Note: the direction-contract prerequisite was dropped; normalisation
    already lives in `resource::direction` and placement does not depend on it.
- [ ] `LOW-EXACT` - exact profile/curve/surface node construction
  - Requires: `LOW-CONTRACT`, `LOW-SESSION`, `INPUT-PROFILE`, `INPUT-MAT`.
  - Implements: `GEOM-PROFILE`, `GEOM-CURVE`, `GEOM-SURFACE`.
  - Proof: focused tests, crate clippy, and relevant declaration/corpus gate.
- [x] `LOW-BREP` - topology plus geometry handles
  - Requires: `LOW-DISPATCH`.
  - Implements: `GEOM-BREP`.
  - Proof: `tests/lower_brep.rs` (10 tests) plus the corpus census.
  - Decision: planar facets carry `surface: None`. The loop's points define the
    plane exactly; fitting one risks disagreeing with the vertices.
  - Decision: vertices intern by source `EntityId`, edges by unordered endpoint
    pair, both scoped per solid. The corpus builds 12 bodies and 2028 faces from
    one 196-point pool, so per-slot emission would multiply vertices ~40x and
    leave every edge unshared, turning closed solids into loose facets.
  - Note: two exact-geometry prerequisites were dropped. Faceted breps need no
    exact curve or surface nodes, so the dependency was theoretical.
- [ ] `LOW-TESS` - preserve authored n-gons/holes/triangles without retessellation
  - Requires: `LOW-DISPATCH`, `INPUT-TOPO`.
  - Implements: `GEOM-TESS`.
  - Proof: focused tests, crate clippy, and relevant declaration/corpus gate.
- [x] `LOW-MAP` - Instance nodes with cycle/depth budgets
  - Implements: `GEOM-MAP`.
  - Proof: `cargo test -p ifc-geometry` (11 mapped tests), crate clippy, corpus census.
  - Decision: `LOW-CONTEXT` was NOT a real prerequisite. A mapped item composes
    world/target/origin frames itself; representation-context selection is a
    product-shape concern that sits above item lowering.
  - Decision: the shared subtree is lowered in the map's own space, so the
    per-occurrence placement rides on the `Instance` transform. That is what
    lets many occurrences reuse one subtree.
- [x] `LOW-PROV` - separate NodeId-to-IFC provenance map
  - Requires: `LOW-SESSION`.
  - Proof: `tests/lower_provenance.rs` covers real multi-entity subtrees,
    innermost active scopes, unscoped nodes, and memo reuse; 5/5 mutation
    probes plus crate clippy and the full gate.
  - Decision: the side table is partial. Nodes emitted for an IFC entity are
    attributed; caller-synthesized unscoped nodes stay unattributed rather than
    receiving a fabricated entity id.
- [ ] `LOW-CENSUS` - lower every supported corpus item and classify every unsupported item
  - Requires: `LOW-DISPATCH`.
  - Implements: `GEOM-CENSUS`.
  - Proof: focused tests, crate clippy, and relevant declaration/corpus gate.

## Completion log

Append `TASK-ID - proof - material decision`; keep long logs out of this file.

- `LOW-MAP` - 11 mapped tests green, 6/6 mutation probes killed, corpus
  dispatch 25 -> 43 lowered - instancing is preserved as `Instance` over a
  shared subtree; transform order is `world o target o origin` with units
  applied once per frame.

- `LOW-SESSION` - 413 tests pass; 4/4 mutation probes caught (cycle, depth,
  dispatch reason, profile memo) - family lowerers now append into one caller
  owned builder and return `NodeId`; `finish` is the only freeze point.
- `LOW-DISPATCH` - corpus census above - unimplemented families are declared
  data with stated reasons rather than a wildcard no-op.
