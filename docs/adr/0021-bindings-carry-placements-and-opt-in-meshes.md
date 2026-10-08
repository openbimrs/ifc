# 0021 — Bindings carry placements by default and meshes on request

- **Status:** Accepted
- **Date:** 2026-10-08
- **Deciders:** openbimrs contributors
- **Supersedes:** —
- **Amends:** [ADR 0013](/adr/0013-language-bindings-wrap-the-facade) — the bindings gain geometry; [ADR 0004](/adr/0004-geometry-bridge-not-kernel) stands unchanged

## Context

The language bindings ([ADR 0013](/adr/0013-language-bindings-wrap-the-facade))
carried no geometry: no placements, no representations, no meshes. For a
browser viewer, the most common reason to load IFC in JavaScript, that was
the main gap ([#328](https://github.com/openbimrs/ifc/issues/328)).

Three levels were possible, each with a different cost:

1. **Placements and representation selection.** Per product, its world
   placement and which representation is the Body. Kernel-free: the facade's
   `geometry-select`, which reads `ifc-model` slots
   ([ADR 0018](/adr/0018-kernel-free-geometry-is-a-feature-not-a-crate)).
2. **The neutral representation as a value.** The `GeometryGraph` each Body
   lowers into, serialised for a host to evaluate with its own kernel.
3. **Meshes.** Triangles from `ifc-geometry`'s `compile-reference-backend`,
   the one execution provider [ADR 0004](/adr/0004-geometry-bridge-not-kernel)
   admits, and only behind an opt-in feature.

The IFC crates never tessellate (ADR 0004): a mesh exists only where the
reference backend ([ADR 0012](/adr/0012-geometry-backends-are-swappable))
computes it, and nothing links that backend by default.

## Decision

We will bind **Level 1 in every default build and Level 3 behind an opt-in
feature in every host**, and defer Level 2.

- **The facade grows both first.** `ifc::product_placements(model, ids)`
  joins `products_world_transforms` with `select_shape_representation` and
  the representation's context, one answer per product with each half's
  own `GeometryResult`. `ifc::product_meshes(model, ids)` (facade feature
  `mesh`: `geometry` plus `ifc-geometry/compile-reference-backend`) calls
  `compile_product_mesh_with` once per product with one backend instance.
  Both take the placement from the public placement API, so a change to
  placement resolution reaches the bindings unchanged. `ifc-geometry`
  re-exports `Tolerance` and `TriMesh` from its `compile` module, so the
  facade names no `axiolid-*` crate.
- **Matrices are 4x4 column-major, in metres.** The layout WebGL, three.js
  (`Matrix4.fromArray`) and most graphics APIs read: three basis columns,
  then the origin.
- **Mesh positions are `f32` relative to the product's placement; the
  placement is `f64`.** The backend returns world coordinates. A
  georeferenced site sits kilometres from the origin, where an `f32` keeps
  centimetres at best, so each vertex is mapped back through the inverse
  placement in `f64` before it is narrowed. A host draws `transform *
  position` and keeps the large offset in the matrix. Indices are `u32`.
  The tolerance is one millimetre: lowering converts every length to
  metres, so the bridge, not the host, knows the file's scale (ADR 0004).
- **Refusals are records, typed per product.** A product whose placement,
  selection, lowering or compilation is refused carries a
  `GeometryRefusal` (`code`, the `entity` at fault, `message`) and every
  other product is still answered. The codes are the shared binding codes:
  `unsupported` (valid IFC not interpreted, or a mesh the backend refused),
  `invalid-model`, `missing-reference` and `budget-exceeded` (a cycle, or a
  chain or aggregate over its limit). A call fails as a whole only with
  `unsupported-schema` (the release is not bundled, as for the domain
  views) or `feature-disabled`.
- **One feature name per level, in every host.** `placements` is a default
  feature of `openbim-ifc-binding-core` and the WASM crate, and so of the
  npm package, the wheel and the C library. `mesh` is opt-in in every host
  crate (`openbim-ifc-wasm`, `openbim-ifc-py`, `openbim-ifc-capi`); without
  it the mesh call exists and refuses with `feature-disabled`, so every
  build has one surface (ADR 0013, #244). No published artifact enables
  it: the npm package stays one module per target, as #317 and #318 left
  it, and publishing a mesh entry is
  [#369](https://github.com/openbimrs/ifc/issues/369).
- **Each host carries arrays in its own idiom.** JavaScript:
  `Float32Array` and `Uint32Array` copies on the record. Python:
  `array('f')` and `array('I')`. C: compiling is the expensive step and C
  sizes every buffer with a first call, so `model_product_meshes` compiles
  once into an opaque mesh-set handle, read record by record
  (`meshes_records`, a tape) and array by array (`meshes_positions`,
  `meshes_indices`, caller buffers), then destroyed; it owns its data and
  outlives the model. .NET wraps those calls (`ProductMeshes`, returning
  `MeshedProduct` with `float[]` and `uint[]`). ABI 0.1.7.

**Level 2 is deferred.** The neutral graph is Axiolid's model, and Axiolid
has not promised it as a stable serialisation format: `axiolid-model` has
no documented encoding, and nothing says which changes to its node kinds
break a serialised graph. Binding it now would freeze Rust internals into
four host APIs. The follow-up is
[#367](https://github.com/openbimrs/ifc/issues/367), blocked by
[axiolid/kernel#267](https://github.com/axiolid/kernel/issues/267).

## Alternatives considered

| Option | Why not |
| --- | --- |
| Meshes in the default build | Links an execution provider into every consumer, which ADR 0004 forbids, and roughly doubles the browser module (see Consequences). |
| Mesh positions in world coordinates | Exact for a model at the origin, wrong by centimetres for a surveyed site once a GPU reads `f32`. |
| Positions as `f64` | Twice the memory and upload; GPUs read `f32`. The `f64` part is the per-product matrix, sixteen numbers. |
| Abort the whole call on the first refused product | One malformed wall would hide the building; the facade's batch placement already answers per product. |
| A second npm build now | #317/#318 kept one module per target; a second module doubles the package. A separate entry is #369. |
| Level 2 now, with Axiolid's Rust types as the format | Couples four host APIs to an unpromised internal model. |

## Consequences

**Positive**

- A viewer gets placements and the chosen Body from every published
  binding, and meshes from a build that asks for them, with the same codes
  in every host.
- The bridge still computes nothing: Level 1 reads slots, and Level 3 calls
  the reference backend ADR 0004 and ADR 0012 already admit.

**Negative / costs**

- Measured on the browser module, after `wasm-bindgen`, without
  `wasm-opt`: Level 1 adds 42 KB to the default
  npm module (2,867,002 to 2,909,468 bytes; +15 KB under `gzip -9`, +11 KB
  under brotli) and 60 KB to an IFC4-only one (815,769 to 875,610). Level 3
  adds 2.1 MB to the default (to 5,052,660 bytes; 978,432 to 1,735,307
  under `gzip -9`, 640,878 to 1,150,357 under brotli) and 2.75 MB to an
  IFC4-only build (to 3,626,301), the reference compiler and its boolean
  engine. The table is in
  [Module size](/bindings/javascript#module-size).
- From npm, meshes need the crate built from source until #369.

**Follow-ups / risks to watch**

- Level 2 waits on axiolid/kernel#267 (#367).
- A host that wants a different backend or tolerance has only the Rust API
  for now; the bindings fix the reference backend and one millimetre.

## Amendments

- *Amended 2026-10-08 (#369): the npm package carries a mesh entry.* The
  package `@openbim/ifc` now ships a second module, the default features
  plus `mesh`, as the subpath entry `@openbim/ifc/mesh` (`mesh/`,
  `mesh/bundler/`, `mesh/web/`, with the default entry's conditional
  exports). The default entry is unchanged, still without meshes, so an
  application that does not import the mesh entry loads none of it; the
  package grows from 3.9 MB to 9.2 MB packed. `mesh` stays opt-in in every
  crate, and the wheel and the C library still leave it out. A second
  package was the alternative: it would duplicate the catalog files and
  their loader, and need its own trusted-publishing configuration and
  version lockstep. `scripts/check-mesh.sh` is gone; the mesh entry's suites
  run in `scripts/build-npm-pkg.sh`. Sizes are in
  [Package size](/bindings/javascript#package-size).

## Relation to existing code

- `crates/openbim-ifc/src/product_geometry.rs` and its `mesh` submodule; the
  facade feature `mesh`.
- `crates/ifc-geometry/src/compile.rs`: the `Tolerance` and `TriMesh`
  re-exports.
- `crates/openbim-ifc-binding-core/src/geometry.rs` and
  `tests/geometry.rs`; features `placements` and `mesh`.
- `crates/openbim-ifc-wasm/src/geometry.rs`,
  `examples/viewer/`, `tests/js/geometry.mjs`, `scripts/build-npm-pkg.sh`
  (the mesh entry, #369).
- `crates/openbim-ifc-py/python/openbim_ifc/geometry.py`,
  `tests/python/test_geometry.py`.
- `crates/openbim-ifc-capi/src/geometry.rs`, the `MESHES` registry,
  `tests/c/smoke.c` (`geometry()`).
- `crates/openbim-ifc-dotnet/dotnet/OpenBim.Ifc/Domains/Geometry.cs`,
  `MeshedProduct.cs`.
- `test/fixtures/synthetic-bindings/binding_geometry.ifc`.
