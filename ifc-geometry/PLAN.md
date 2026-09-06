# ifc-geometry implementation plan

Status: active implementation on a shared lowering session; BRep/shell, tessellated, swept/boolean, elementary and bounded surfaces, and major curve/representation-selection paths lower to neutral Axiolid graphs.
Last updated: 2026-09-01

This is task state, not ambient context. Follow `AGENTS.md`; claim one task ID,
record blockers/decisions under it, and check it off only with evidence.

## Established boundary

Interpret all shape-affecting IFC data and lower exact intent into a format-neutral axiolid-model DAG.

## Planned file map

The paths below already compile as private scaffold owners. Replace each
planned-owner marker with its first real view, contract, and tests; do not add
parallel placeholders.

- `src/input/profile.rs`: IfcProfileResource shape slots and local 2D position
- `src/input/representation.rs`: Body/Axis/FootPrint selection and contexts
- `src/input/material_usage.rs`: profile/layer offsets and cardinal-point geometry inputs
- `src/input/product.rs`: product shape and local-placement links
- `src/input/topology.rs`: IfcTopologyResource views used by B-rep
- `src/lower/session.rs`: shared builder, memoization, active stack, and provenance
- `src/lower/dispatch.rs`: total representation-item dispatcher
- `src/lower/context.rs`: representation context and precision policy
- `src/lower/curve.rs`: exact curve graph nodes
- `src/lower/surface.rs`: exact surface graph nodes
- `src/lower/brep.rs`: topology plus geometry handles
- `src/lower/tessellated.rs`: preserve n-gons/holes and explicit input triangles
- `src/lower/mapped.rs`: DAG Instance nodes with cycle/depth budgets
- `src/lower/boolean.rs`: exact operation trees and half spaces
- `src/lower/provenance.rs`: NodeId to IFC source side table

- `src/input/material_usage.rs`: compiled private scaffold; implementation owned by `src/input/PLAN.md`
- `src/input/product.rs`: compiled private scaffold; implementation owned by `src/input/PLAN.md`
- `src/input/profile.rs`: compiled private scaffold; implementation owned by `src/input/PLAN.md`
- `src/input/representation.rs`: compiled private scaffold; implementation owned by `src/input/PLAN.md`
- `src/input/topology.rs`: compiled private scaffold; implementation owned by `src/input/PLAN.md`
- `src/lower/brep.rs`: compiled private scaffold; implementation owned by `src/lower/PLAN.md`
- `src/lower/context.rs`: compiled private scaffold; implementation owned by `src/lower/PLAN.md`
- `src/lower/curve.rs`: compiled private scaffold; implementation owned by `src/lower/PLAN.md`
- `src/lower/mapped.rs`: compiled private scaffold; implementation owned by `src/lower/PLAN.md`
- `src/lower/placement.rs`: compiled private scaffold; implementation owned by `src/lower/PLAN.md`
- `src/lower/solid.rs`: compiled private scaffold; implementation owned by `src/lower/PLAN.md`
- `src/lower/surface.rs`: compiled private scaffold; implementation owned by `src/lower/PLAN.md`
- `src/lower/tessellated.rs`: compiled private scaffold; implementation owned by `src/lower/PLAN.md`

## Work queue

- [x] `GEOM-PLACE-API` - expose product placement as a first-class kernel-free API.
  - Evidence: 7 unit tests, 3/3 mutation probes, and a differential run against
    `apps/open-signs` showing 127 placements with 0 disagreements.

- [x] `GEOM-CONTRACT` - enforce validated direction/frame invariants at the IFC-to-Axiolid boundary
  - Contract: axes and normals become finite non-zero unit directions; surface
    normals use inverse-transpose affine transformation; magnitude-bearing
    vectors are not normalized.
  - Evidence: scale-safe extreme/zero/non-finite direction tests, ordinary and
    extreme finite inverse-transpose normal tests including scale-aware sheared
    determinant orientation, base-axis tests, checked neutral-frame tests, and
    both feature columns.
- [x] `GEOM-SESSION` - introduce one recursive lowering session and shared graph builder
  - Evidence: `cargo test -p ifc-geometry` (413 passing) plus 4/4 mutation
    probes; `tests/lower_session.rs` proves boolean composition across families,
    shared-profile reuse, frame-distinct memo keys, cycle/depth limits, and
    entity-attributed graph faults. Owned by `src/lower/PLAN.md:LOW-SESSION`.
  - Decision: `GEOM-CONTRACT` was not an actual prerequisite; the session is
    agnostic to direction normalization, which already lives in
    `resource::direction`.
- [x] `GEOM-SEAM` - remove stale kernel compatibility and adapter-owned approximation policy
  - Evidence: deleted compatibility/tolerance modules, public API regression, package tests, and clippy in both feature columns.
- [x] `GEOM-INPUT` - add cross-resource input views without importing semantic domain crates
  - Audited (2026-09-05): all five `src/input/PLAN.md` sub-tasks are complete.
    The profile/topology slices were resolved by ownership in #25: their slots
    are owned by `lower::profile` and `resource::topology`/`solid::brep`, so no
    `input` module for them exists by design.
- [x] `GEOM-CTX` - select shape representations and compose geometric contexts/precision
  - Done (2026-09-05): representation selection, sub-context inheritance and
    precision were already implemented. The gap was that
    `WorldCoordinateSystem` -- a MANDATORY context attribute -- was read and
    tested but never applied, so a file surveying its site into a real
    coordinate system lowered every product at the wrong place. The context
    frame now composes above the placement chain in `lower::context`.
  - Evidence: `tests/lower_product.rs` (6 passing) plus 2/2 mutation probes:
    dropping the frame and composing it in the wrong order both fail.
- [x] `GEOM-PLACE` - compose units, local placements, item frames, and provenance exactly once
  - Requires: `GEOM-SESSION`.
  - Evidence: `tests/lower_product.rs`, the ifc-cli corpus placement gate, and
    4/4 mutation probes including the original all-products-at-origin bug.
  - Note: source attribution is now implemented by the session side table;
    placement remains responsible only for units and frame composition.
- [x] `GEOM-PROFILE` - cover exact profile families, local profile Position, voids, and material cardinal offsets
  - Audited (2026-09-05): all four named concerns are implemented. All 21
    concrete IFC4 `IfcProfileDef` subtypes lower; abstract `IfcProfileDef`
    itself is correctly refused. Local `Position`, arbitrary profiles with
    voids, and material cardinal offsets all have code and tests.
  - Evidence: `tests/lower_profile_families.rs` (13 passing), including a
    schema-driven gate that fails if the schema gains a concrete family with
    no disposition. The box was held open only by stale prerequisite state.
- [x] `GEOM-CURVE` - lower every concrete curve family without approximation
  - Audited (2026-09-05): every concrete curve family lowers or carries a
    named typed refusal. Polyline, indexed poly-curve (including exact
    three-point arcs in both world and parameter space), line, circle,
    ellipse, trimmed, composite, explicit-knot B-spline, offset, p-curve and
    surface-curve graphs all lower. `IfcPointOnCurve` preserves its basis and
    parameter. `PCurveS1`-vs-`S2` master selection now lowers on axiolid
    v0.11.0 (#24).
  - Remaining refusals are correct-by-design, not gaps: a convention-only
    `IfcBSplineCurve` has no authored knots to preserve, and a parameter-space
    conic on an `IfcAxis2Placement3D` would need an invented projection.
    Parameter-space trimmed/composite curves stay refused pending a dimensional
    contract for mixed-domain trim parameters.
  - Requires: `GEOM-CONTRACT`, `GEOM-SESSION`.
  - Evidence: focused unit/property/fixture tests, isolated build, and crate clippy.
- [x] `GEOM-SURFACE` - lower elementary, swept, bounded, and B-spline surfaces
  - Audited (2026-09-05): all four groups lower; surface/PLAN.md sub-tasks are
    complete except SURF-ELEM, which is a typed-placement-view refactor rather
    than missing lowering capability.
  - Requires: `GEOM-CONTRACT`, `GEOM-SESSION`, `GEOM-CURVE`.
  - Progress: all four groups now lower (`LOW-EXACT`) - elementary (plane,
    cylinder, sphere, torus), swept (linear extrusion, revolution), bounded
    (rectangular-trimmed, curve-bounded plane/surface) and B-spline. The curved and
    B-spline families were fixture-blocked rather than effort-blocked; they
    are now covered by generated fixtures, see `src/lower/PLAN.md`.
    `IfcPointOnSurface` lowers to `axiolid_model::PointOnSurface`, preserving
    the basis surface reference and both `(u, v)` parameters through the same
    per-basis-kind unit conversion trims already use (2026-09-05).
  - Evidence: focused unit/property/fixture tests, isolated build, and crate clippy.
  - Completed slice: explicit-knot polynomial/rational tensor-product B-splines
    preserve both axes and control/weight nets and match scalar-oracle
    evaluation; broader surface conformance and test depth remain open.
- [x] `GEOM-BREP` - lower topology and 20 corpus faceted B-reps
  - Requires: `GEOM-SESSION`.
  - Evidence: `tests/lower_brep.rs`; corpus census rose 43 -> 64 lowered items
    and `IFCFACETEDBREP` left the unsupported set entirely. Cube fixture checks
    V - E + F = 2; the 12-solid shared-point fixture lowers all 2028 faces.
    9/9 mutation probes killed. Advanced B-reps now lower through the same
    path, with support curves and surfaces attached and edge sharing preserved
    across oriented-edge reuse; 7/7 further mutation probes.
- [x] `GEOM-TESS` - lower tessellated and polygonal face sets without forced triangulation
  - Evidence: 7 unit + 3 fixture tests; corpus census rose 64 -> 67 lowered and
    `IFCTRIANGULATEDFACESET` left the unsupported set; both feature columns
    build and pass clippy; 6/6 mutation probes caught.
  - Decision: `GEOM-INPUT` was not required. The tessellated views already
    existed under `solid::tessellated` and depend only on `error` and `slots`.
- [x] `GEOM-SOLID` - complete exact solid families
  - Audited (2026-09-05): every `src/solid/PLAN.md` sub-task is complete, and
    `IfcPolygonalBoundedHalfSpace` (#20) cleared the last PLANNED family, so
    the lowering PLANNED table is now empty.
  - Progress: booleans, CSG primitives/solids, swept disks, tapered/fixed-reference sweeps, sectioned spines, surface models, advanced/faceted B-reps with voids, and unbounded/boxed half spaces lower exactly.
  - Blocker: `IfcPolygonalBoundedHalfSpace` remains typed unsupported. Axiolid
    added the required `SolidOperation::BoundedHalfSpace` contract (2026-08-30,
    commit `9de5042`) and fixed the `agreement=false` mirroring defect
    (axiolid/kernel#83, fixed in `6fe7bf3`/v0.9.0, independently re-verified).
    A second gap remains: `bounded_half_space` derives the boundary's in-plane
    basis solely from the clip plane's normal, with no way to supply an
    independent orientation -- but `IfcPolygonalBoundedHalfSpace.Position` is
    schema-independent of `BaseSurface` and can differ from Axiolid's guess.
    Wiring this crate to the contract before that lands would silently rotate
    the clip for any file whose `Position.RefDirection` disagrees with the
    guess. See `src/lower/PLAN.md` and axiolid/kernel#93.
- [x] `GEOM-MAP` - preserve mapped-item instancing with cycle/depth limits
  - Evidence: 11 mapped-item tests over real fixtures, 6/6 mutation probes,
    isolated build, and crate clippy.
  - Decision: `GEOM-PLACE` was not required. Mapped items compose their own
    frames; product-level placement composition remains open under that task.
- [x] `GEOM-SPLIT` - make the neutral geometry crates optional for 2D consumers
  - Evidence: `tests/kernel_free_build.rs` plus `openbim-ifc/tests/thin_build.rs`
    check the resolved dependency graph in both feature columns; 4/4 mutation
    probes caught. See the completion log for measured crate counts.
- [x] `GEOM-PLAN-SELECT` - intersect plan context and drawable identifier
  - Evidence: `tests/representation_context.rs` (18 passing) including two
    ArchiCAD bounding-box regressions; measured against a real export.
- [x] `GEOM-CENSUS` - reconcile concrete IFC4 declarations, ownership, dispatch, corpus, and behavior
  - Evidence: executable representation-item disposition ledger plus exact/planned corpus gates and profile-family census.
  - Mutation proof: declaration, corpus-instance, curve-semantic, and polygonal-bound mutations each failed with exit 101.

## Completion log

`GEOM-CONTRACT` / `GEOM-SEAM` / `GEOM-CENSUS` - scale-safe frame and direction
contracts and executable IFC4 ledgers. `./scripts/gate.sh` passed on 2026-09-01.

`GEOM-PLACE-API` - `cargo test -p ifc-geometry --no-default-features --lib placement`
7 passing; `--test kernel_free_build` 3 passing. Moved
`product_world_transform` out of `lower::context` into `constraint::placement`
and re-exported it at the crate root and the facade.

Material finding: the function already existed with the signature FINDINGS.md
F-03 asked for, but it was unreachable -- never re-exported at the crate root,
and after the `lowering` split it did not compile at all in the kernel-free
column. F-03 read as "not implemented" when the truth was "implemented,
unreachable". A differential probe against the app that motivated the finding
resolved 127 placements with 0 disagreements, so this is a reachability fix,
not a reimplementation.

Added `products_world_transforms` for the batch case: products in a storey
share the whole storey-building-site tail, and the existing `PlacementResolver`
cache was being discarded once per product.

`GEOM-SPLIT` - `cargo test -p ifc-geometry --test kernel_free_build`,
`cargo test -p openbim-ifc --test thin_build` (8 passing), both feature columns
green; 4/4 mutation probes caught - the neutral crates are optional behind the
default-on `lowering` feature. Measured 26 crates with lowering, 17 without;
all eight `axiolid-*` crates and `glam` drop out. Only `lower`,
`Transform::to_geom` and the `IfcBooleanOperator` conversion may name them.
Decision: a feature, not a crate split. `lower` is already a leaf module -- no
other module references it -- so the separation holds structurally and a gate
costs one test, where a second crate would cost a published name plus a
manifest edit for every consumer. Revisit if the kernel-free half grows its own
heavy dependencies or needs a separate release cadence.

`GEOM-PLAN-SELECT` - `cargo test -p ifc-geometry --test representation_context`
(18 passing), probe on `AC20-FZK-Haus.ifc`; 1/1 mutation probe caught -
`select_plan_representation` intersects context and identifier instead of
ordering them. Evidence: 107 of 253 shape representations in that file are
`Box`/`BoundingBox` authored inside a `PLAN_VIEW` sub-context, so the
context-first rule returned boxes for every one of 121 resolving products.
After the fix, 34 products resolve (14 `Annotation`, 13 `Axis`, 7 `FootPrint`)
and none are non-drawable. Decision: the drop from 121 to 34 is correct, not a
regression -- those products have only a bounding box, and `None` is the
documented answer for "no drawable plan geometry".


`GEOM-CTX` - `cargo test -p ifc-geometry` (23 context tests); 14/14 mutation
probes caught - representation contexts and the 2D selector. Slot constants
asserted against IFC2x3/IFC4/IFC4x3. DERIVED (`*`) attributes resolve through
`ParentContext`; the depth bound is the single termination mechanism and is
tested at both edges (ADR 0009).

Append concise entries as `TASK-ID - proof command/result - material decision`.

- `GEOM-SESSION` - `cargo test -p ifc-geometry` 413 passing, 4/4 mutation
  probes caught - recursive lowering shares one builder; `LoweredGeometry` is
  produced only by `LoweringSession::finish`.
- `GEOM-CURVE` NURBS slice - `nurbs_import` and `nurbs_lowering` parse a synthetic
  IFC4 rational curve/surface fixture through `openbim-step`/`ifc-step`, assert
  degrees, compact knots, multiplicities, controls, and weights, then match
  scalar-oracle evaluations. Explicit-knot polynomial/rational subtypes are
  implemented; base convention-only splines and complete curve/surface work
  remain open.
- `GEOM-CTX/CURVE/SURFACE` - `cargo +1.88.0 test -p ifc-geometry` and
  `lower_dispatch_corpus` pass with a committed IFC fixture covering ellipse,
  offset, indexed line/arc, p-/surface-curve, curve-bounded surface and explicit
  Body/Plan selection; unsupported exact forms remain typed refusals.
Do not paste long logs or move standing invariants out of `AGENTS.md`.
