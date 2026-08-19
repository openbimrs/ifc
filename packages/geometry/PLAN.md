# Geometry package plan

Status: active architecture scaffold
Last updated: 2026-08-19

This is implementation planning, not ambient agent context. Read `AGENTS.md`
for standing rules. Read this file only when planning or implementing geometry
capabilities.

## 1. Goal

Build a pure-Rust, IFC-agnostic geometry package family that:

- is useful as small independent crates;
- supports every geometric concept needed by `ifc-geometry` without importing
  IFC names or schema types;
- defaults to a portable, deterministic mesh-first path;
- makes exact topology, NURBS, robust booleans, parallel execution, SIMD, and
  GPU execution additive capabilities rather than mandatory dependencies;
- can absorb useful algorithms from the Solibri sibling through adapters and
  ports, without preserving its domain vocabulary or monolithic crate shape;
- permits future x86_64, AArch64, discrete-GPU, integrated-GPU, and foreign
  kernel implementations behind stable contracts.

## 2. Non-negotiable constraints

- No C++ or system geometry library in the dependency graph.
- No IFC, rendering, persistence, product-domain, GUID, or rule-engine types in
  `packages/geometry/`.
- Units and tolerances are explicit values. No process-global epsilon.
- The portable scalar implementation is always available as correctness oracle.
- Feature-disabled configurations compile and test independently; workspace
  feature unification is not accepted as proof.
- Capability absence is data (`Unsupported` / capability query), never a panic
  or silent approximation.
- Invalid and dirty geometry remains diagnosable and, where safe, representable.
- Performance claims require benchmarks. Optimized backends require
  differential tests against the scalar oracle.
- Public data types implement the standard traits clients reasonably expect;
  floating-point types do not claim `Eq` or `Hash` unless canonicalized.

## 3. Current unknowns to close before freezing the graph

- Exact responsibility mapping for all declarations in IFC4 ADD2 TC1
  `IfcGeometryResource`, `IfcGeometricModelResource`, and
  `IfcGeometricConstraintResource` (112 entities, 23 types, 28 functions).
- Which declarations need a neutral geometry value, which are IFC-only views,
  and which are schema validation/derived functions rather than kernel work.
- Whether backend implementations belong inside `geom-kernel` features or in
  leaf backend crates; feature-unification evidence currently favors separate
  backend crates.
- Minimum dependency set for robust 2D profile work and pure-Rust mesh boolean.
- GPU contract granularity: whole batch / mesh buffers, never per triangle.

## 4. Planned layers

```text
L0 policy/math       geom-core
L1 values            geom-mesh, geom-profile, geom-curve, geom-surface,
                     geom-topology, geom-model
L2 algorithms        geom-primitive, geom-sweep, geom-tessellate,
                     geom-spatial, geom-measure, geom-heal
L3 contracts         geom-kernel (capabilities, requests, reports, traits only)
L4 implementations   backend crates or feature-isolated implementation modules
L5 format bridges    ifc-geometry, future STEP-CAD/CityGML/etc. (outside here)
```

Dependencies point downward. Same-layer edges require a documented reason.
Representation crates never depend on algorithms or backends.

## 5. Capability tiers (target user experience)

- `core`: math, transforms, bounds, tolerances, diagnostics; tiny and portable.
- `mesh`: mesh data, validation, normals, deterministic canonicalization.
- `basic`: profiles, primitives, extrusion/revolution, tessellation of analytic
  forms, measurement; enough for common building IFC.
- `spatial`: BVH and batch queries; optional Rayon parallel execution.
- `boolean`: pure-Rust mesh CSG behind a coarse batch trait.
- `advanced`: NURBS evaluation, exact topology, advanced B-rep tessellation.
- `gpu`: optional GPU implementation selected by capability at runtime.
- architecture-specific acceleration is provided by backend implementation,
  not by changing public geometry types or compiling for the developer CPU.

A facade may offer these bundles later. Individual crates remain directly
usable, and no facade feature is allowed to hide undeclared backend coupling.

## 6. Workstreams

1. Generate an authoritative IFC declaration manifest from local specification
   HTML and map every declaration to IFC bridge and neutral geometry ownership.
2. Replace the duplicate IFC-local primitive vocabulary with `geom-model`,
   preserving compatibility through explicit re-exports only where justified.
3. Split each crate's root into growth-shaped modules with honest status docs.
4. Define compact capability traits and request/report data; avoid a god trait.
5. Separate portable algorithms from execution policy and hardware backends.
6. Add progressive `AGENTS.md` files at ownership boundaries.
7. Add architecture, feature-isolation, API-trait, orphan-module, and schema
   coverage gates; mutation-test the gates.
8. Add benchmark harnesses but do not claim wins before measurements exist.

## 7. Validation strategy

- `cargo build/test/clippy/doc` for the full workspace and relevant feature
  combinations, checking command exit codes.
- Isolated builds for every bridge, contract, and backend crate.
- `cargo tree -e features` assertions for lean configurations.
- Compile-fail or manifest/source architecture tests for forbidden edges.
- Generated declaration coverage test: 112 entities + 23 types + 28 functions,
  with no hand-trimmed allowlist.
- Stub/foreign kernel implementation proving contracts are implementable.
- Differential scalar vs SIMD/parallel/GPU tests once implementations exist.
- Determinism tests across process boundaries for output ordering and caches.

## 8. Risks and rollback

- Too many crates: keep crates at independently useful compile/dependency
  boundaries; use modules for ordinary code organization.
- Feature explosion: features name additive capabilities, never products or
  hardware models. Prefer separate backend crates over mutually interacting
  feature flags.
- False 100% claim: distinguish `represented`, `interpreted`, `evaluated`, and
  `validated`; report each separately.
- Premature exact B-rep: keep the seam, implement mesh-first, and require corpus
  evidence before funding surface/surface intersection machinery.
- Shared master: stage paths narrowly and verify HEAD before every commit.

## 9. Next concrete action

Generate the authoritative three-resource declaration manifest and compare it
with compiled `ifc-geometry` API coverage before changing the public contracts.
