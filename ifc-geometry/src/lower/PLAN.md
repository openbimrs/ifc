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
- [x] `LOW-CURVE` - exact curve nodes for the families used as directrices
  - Scope: the directrix curve families of the curve parent task; that
    parent stays open for B-splines, ellipses and offset curves.
  - Proof: `cargo test -p ifc-geometry` (6 curve unit tests, 3 corpus tests in
    `tests/lower_csg_swept.rs`); 9/9 mutation probes; crate clippy in both
    feature columns.
  - Scope: `IfcPolyline`, `IfcLine`, `IfcCircle`, `IfcTrimmedCurve`,
    `IfcCompositeCurve`. `IfcBSplineCurve*`, `IfcEllipse`, `IfcOffsetCurve*`
    and `IfcIndexedPolyCurve` still report a typed `Unsupported`: the corpus
    does not exercise them and a guessed parameterisation is worse than a
    stated gap.
  - Decision: `LOW-CONTRACT` was NOT a prerequisite. Direction normalization
    already lives in `resource::direction`, and this module deliberately does
    NOT normalize an `IfcVector` magnitude, which is parameterisation rather
    than orientation.
  - Decision: a conic trim parameter is an ANGLE and a line parameter is a
    LENGTH. The basis curve therefore selects the unit conversion. A single
    length factor turns the crankbar's 0.082 rad arcs into 8.2e-5 rad in a
    millimetre file; the arc still renders.
- [ ] `LOW-EXACT` - exact profile/surface node construction
  - Requires: `LOW-CONTRACT`, `INPUT-PROFILE`, `INPUT-MAT`.
  - Scope note: the curve third is done, see `LOW-CURVE`. Profiles already
    lower via `lower::profile`. What remains here is exact SURFACE nodes.
  - Done: `IfcPlane` and `IfcSurfaceOfLinearExtrusion` lower via
    `lower/surface.rs`. `Transform::to_geom_frame` carries the placement's own
    U/V axes, so a surface keeps the parameterisation trims are taken against.
    Proof: 7 unit tests, `tests/lower_surface.rs` (3 corpus tests), 6/6
    mutation probes, crate clippy in both feature columns.
  - Blocked, with evidence: `IfcCylindricalSurface`, `IfcSphericalSurface`,
    `IfcToroidalSurface`, `IfcSurfaceOfRevolution` and the B-spline families
    have complete readers in `crate::surface` and kernel variants waiting, but
    no licensed fixture exists. Surveyed 909 `.ifc` files across ifc-lite
    (MPL-2.0), IfcOpenShell (LGPL-3.0), IfcOpenShell/files (NO licence) and
    buildingSMART (CC-BY-4.0): every file carrying these families is in the
    unlicensed repo. They stay in `dispatch::PLANNED` rather than shipping
    lowering code no fixture can exercise.
  - Decision: `IfcArbitraryOpenProfileDef` is reported unsupported with a
    stated reason, not approximated. The neutral profile model is built on
    closed contours; closing the curve would fabricate a face the file never
    described. This is what blocks `IFCSURFACECURVESWEPTAREASOLID`, whose
    lowering is implemented and waiting on it.
- [x] `LOW-CSG` - CSG solids, CSG primitives, and swept-disk solids
  - Requires: `LOW-CURVE`, `LOW-DISPATCH`.
  - Scope: the CSG and swept-disk families of the solid parent task; that
    parent stays open for advanced brep and surface-curve sweeps.
  - Proof: 6 unit tests, `tests/lower_csg_swept.rs` (3 corpus tests), 9/9
    mutation probes. Corpus census rose 72 -> 80 and the unsupported set is
    now EMPTY for the committed corpus.
  - Decision: `IfcCsgSolid` lowers to whatever its `TreeRootExpression` lowers
    to. The wrapper carries no geometry, so emitting a node for it would add a
    graph level no consumer can act on.
  - Decision: a CSG primitive is LOCAL by kernel contract, so its `Position`
    rides on an `Instance` node rather than being folded into the extents.
    Folding would discard the origin offset and break any rotation.
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
- [x] `LOW-TESS` - preserve authored n-gons/holes/triangles without retessellation
  - Implements: `GEOM-TESS`.
  - Proof: 7 unit tests in `lower/tessellated/tests.rs`, 3 fixture tests in
    `tests/lower_tessellated.rs`, and the corpus census (64 -> 67 lowered,
    `IFCTRIANGULATEDFACESET` out of the unsupported set). 6/6 mutation probes.
  - Decision: `INPUT-TOPO` was NOT a real prerequisite. A face set carries no
    adjacency, so the topology views B-rep needs are irrelevant here; the
    tessellated readers import only `error` and `slots`.
  - Decision: these lower to `axiolid-mesh` types, not `BRep`. A face set is
    already a discretisation, so recovering topology would mean inferring
    shared edges by comparing floats -- inventing information the file never
    carried. `PolygonMesh` keeps authored n-gons and holes verbatim so the
    fill rule and tolerance stay with the kernel.
- [x] `LOW-HALFSPACE` - half spaces as boolean cutting tools
  - Scope: the half-space families of the solid parent task; that parent
    stays open for advanced brep and surface-curve sweeps.
  - Proof: `src/lower/halfspace/tests.rs` (6 tests), `tests/lower_halfspace.rs`
    (3 tests over `issue_1155_halfspace_flyaway.ifc`), corpus census 67 -> 72
    lowered, and 6/6 mutation probes.
  - Decision: `GEOM-PROFILE`/`GEOM-SURFACE` were NOT prerequisites. A planar
    base surface needs only `resource::placement`, which already exists. The
    exact-surface dependency applies to curved bases, which are reported as
    unsupported rather than flattened to a tangent plane.
  - Decision: IFC `AgreementFlag` is INVERTED relative to the neutral
    `HalfSpace.agreement`. IFC `.T.` selects the side the normal points away
    from; the kernel's `true` selects the normal side. Transcribing it straight
    through cuts away the half that should have been kept, and no geometric
    check catches it -- the boolean still evaluates and the mesh is still
    watertight.
  - Decision: `IfcBoxedHalfSpace` and `IfcPolygonalBoundedHalfSpace` lower to
    their underlying half space. Both bounds are clipping HINTS; the neutral
    node carries none, and building a prism from an unlowered 2D boundary curve
    would invent geometry. Recorded here rather than approximated.
  - Note: a surviving mutant showed the renormalization after the world
    transform is unreachable with a unit-basis frame, because
    `axis_placement_transform` already normalizes. The test now composes a
    scaling frame so the assertion is load-bearing.
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
