# ifc-geometry implementation plan

Status: active implementation on a shared lowering session; views are broad and dispatch is total, but only swept-solid and boolean families are lowered today.
Last updated: 2026-08-19

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

- [ ] `GEOM-CONTRACT` - agree validated direction/axis invariants with `axiolid-model`
  - Contract: axes, normals, and orientation fields become finite non-zero unit directions; displacement, derivative, scale, and other magnitude-bearing vectors are never normalized implicitly.
  - Evidence: contract docs plus non-unit, zero-vector, and magnitude-preservation tests on both sides.
- [x] `GEOM-SESSION` - introduce one recursive lowering session and shared graph builder
  - Evidence: `cargo test -p ifc-geometry` (413 passing) plus 4/4 mutation
    probes; `tests/lower_session.rs` proves boolean composition across families,
    shared-profile reuse, frame-distinct memo keys, cycle/depth limits, and
    entity-attributed graph faults. Owned by `src/lower/PLAN.md:LOW-SESSION`.
  - Decision: `GEOM-CONTRACT` was not an actual prerequisite; the session is
    agnostic to direction normalization, which already lives in
    `resource::direction`.
- [ ] `GEOM-SEAM` - finish neutral-DAG migration; remove stale kernel-trait wording and obsolete adapter tessellation tolerance
  - Evidence: focused unit/property/fixture tests, isolated build, and crate clippy.
- [ ] `GEOM-INPUT` - add cross-resource input views without importing semantic domain crates
  - Evidence: focused unit/property/fixture tests, isolated build, and crate clippy.
- [ ] `GEOM-CTX` - select shape representations and compose geometric contexts/precision
  - Requires: `GEOM-CONTRACT`, `GEOM-INPUT`.
  - Evidence: focused unit/property/fixture tests, isolated build, and crate clippy.
- [x] `GEOM-PLACE` - compose units, local placements, item frames, and provenance exactly once
  - Requires: `GEOM-SESSION`.
  - Evidence: `tests/lower_product.rs`, the ifc-cli corpus placement gate, and
    4/4 mutation probes including the original all-products-at-origin bug.
  - Note: source attribution is now implemented by the session side table;
    placement remains responsible only for units and frame composition.
- [ ] `GEOM-PROFILE` - cover exact profile families, local profile Position, voids, and material cardinal offsets
  - Requires: `GEOM-CONTRACT`, `GEOM-SESSION`, `GEOM-INPUT`, `GEOM-PLACE`.
  - Evidence: focused unit/property/fixture tests, isolated build, and crate clippy.
- [ ] `GEOM-CURVE` - lower every concrete curve family without approximation
  - Progress: `LOW-CURVE` lowers polyline, line, circle, trimmed and composite
    curves (9/9 mutation probes). B-splines, ellipses, offset curves and
    indexed poly-curves still report a typed `Unsupported`.
  - Requires: `GEOM-CONTRACT`, `GEOM-SESSION`.
  - Evidence: focused unit/property/fixture tests, isolated build, and crate clippy.
- [ ] `GEOM-SURFACE` - lower elementary, swept, bounded, and B-spline surfaces
  - Requires: `GEOM-CONTRACT`, `GEOM-SESSION`, `GEOM-CURVE`.
  - Evidence: focused unit/property/fixture tests, isolated build, and crate clippy.
- [x] `GEOM-BREP` - lower topology and 20 corpus faceted B-reps
  - Requires: `GEOM-SESSION`.
  - Evidence: `tests/lower_brep.rs`; corpus census rose 43 -> 64 lowered items
    and `IFCFACETEDBREP` left the unsupported set entirely. Cube fixture checks
    V - E + F = 2; the 12-solid shared-point fixture lowers all 2028 faces.
    9/9 mutation probes killed.
- [x] `GEOM-TESS` - lower tessellated and polygonal face sets without forced triangulation
  - Evidence: 7 unit + 3 fixture tests; corpus census rose 64 -> 67 lowered and
    `IFCTRIANGULATEDFACESET` left the unsupported set; both feature columns
    build and pass clippy; 6/6 mutation probes caught.
  - Decision: `GEOM-INPUT` was not required. The tessellated views already
    existed under `solid::tessellated` and depend only on `error` and `slots`.
- [ ] `GEOM-SOLID` - complete booleans, halfspaces, CSG, and swept-disk families
  - Requires: `GEOM-SESSION`. `GEOM-PROFILE`/`GEOM-SURFACE` apply only to the
    families that still need exact profiles or curved surfaces.
  - Half spaces: DONE. `src/lower/halfspace.rs`, owned by
    `src/lower/PLAN.md:LOW-HALFSPACE`. Evidence: 9 tests, corpus census
    67 -> 72, 6/6 mutation probes. `IFCBOOLEANCLIPPINGRESULT` left the
    unsupported set as a side effect: its cutting tool now lowers.
  - Remaining: `IFCCSGSOLID` (needs CSG primitive nodes), `IFCSWEPTDISKSOLID`
    and `IFCSURFACECURVESWEPTAREASOLID` (need exact curve lowering),
    `IFCSECTIONEDSPINE`.
  - Progress: booleans, half spaces, CSG solids/primitives and swept disks all
    lower. Corpus census 80 lowered with an EMPTY unsupported set. Remaining
    families in `dispatch::PLANNED` (advanced brep, surface-curve sweep,
    sectioned spine) do not occur in the committed corpus.
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
- [ ] `GEOM-CENSUS` - keep declaration and real-corpus lowering coverage executable
  - Contract: record one implementation owner per unique declaration separately from many-to-many IFC resource memberships; do not double-count `IfcSameAxis2Placement`, `IfcSameCartesianPoint`, `IfcSameDirection`, or `IfcSameValue`.
  - Evidence: focused unit/property/fixture tests, isolated build, and crate clippy.

## Completion log

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
all eight `axiolid-*` crates and `glam` drop out. Only `lower`, `kernel`,
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
Do not paste long logs or move standing invariants out of `AGENTS.md`.
