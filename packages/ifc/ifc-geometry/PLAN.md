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
- [ ] `GEOM-TESS` - lower tessellated and polygonal face sets without forced triangulation
  - Requires: `GEOM-SESSION`, `GEOM-INPUT`.
  - Evidence: focused unit/property/fixture tests, isolated build, and crate clippy.
- [ ] `GEOM-SOLID` - complete booleans, halfspaces, CSG, and swept-disk families
  - Requires: `GEOM-SESSION`, `GEOM-PROFILE`, `GEOM-SURFACE`.
  - Evidence: focused unit/property/fixture tests, isolated build, and crate clippy.
- [x] `GEOM-MAP` - preserve mapped-item instancing with cycle/depth limits
  - Evidence: 11 mapped-item tests over real fixtures, 6/6 mutation probes,
    isolated build, and crate clippy.
  - Decision: `GEOM-PLACE` was not required. Mapped items compose their own
    frames; product-level placement composition remains open under that task.
- [ ] `GEOM-CENSUS` - keep declaration and real-corpus lowering coverage executable
  - Contract: record one implementation owner per unique declaration separately from many-to-many IFC resource memberships; do not double-count `IfcSameAxis2Placement`, `IfcSameCartesianPoint`, `IfcSameDirection`, or `IfcSameValue`.
  - Evidence: focused unit/property/fixture tests, isolated build, and crate clippy.

## Completion log

Append concise entries as `TASK-ID - proof command/result - material decision`.

- `GEOM-SESSION` - `cargo test -p ifc-geometry` 413 passing, 4/4 mutation
  probes caught - recursive lowering shares one builder; `LoweredGeometry` is
  produced only by `LoweringSession::finish`.
Do not paste long logs or move standing invariants out of `AGENTS.md`.
