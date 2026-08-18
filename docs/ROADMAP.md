# Roadmap

**Mission:** the best IFC library in Rust — a lightweight, high-performance
alternative to IfcOpenShell, without OpenCascade.

Validation-gated stages. No stage is "done" on a claim. Every stage lands with
(a) a cross-check against `test/fixtures/` (or a larger oracle corpus) and
(b) a measured wall-clock. Performance claims are always backed by a benchmark
number, never asserted.

Hardware on the dev box: Intel Xeon w7-3565X, 20 vCPU, AVX-512
(f/dq/bw/vl/vbmi/ifma/cd) + AMX (tile/int8/bf16), 62 GB RAM.

## Stage 0 — Architecture scaffold ✅ DONE

- [x] Role-grouped layout: `packages/{geometry,ifc,openbim}`, `bindings/`,
      `apps/` — 17 crates, one-way dependency direction.
- [x] `geom-kernel` trait contract (`MeshBoolean`, `Capabilities`, `GeomError`)
      plus `backend::{scalar,simd,gpu}` behind cargo features and
      `backend::Dispatcher` runtime selection.
- [x] SIMD runtime feature detection (`is_x86_feature_detected!`), no
      compile-time hardware lock-in.
- [x] `ifc-geometry` seam generic over `K: MeshBoolean`; proven backend-agnostic
      by a test injecting a kernel that is not one of ours.
- [x] `apps/ifc-cli` runs: `ifc capabilities` reports detected backends and
      honestly reports no boolean implementation.
- [x] ADRs 0001–0004.
- [x] **Validated:** `cargo build/test/clippy -D warnings/fmt --check/doc` all
      green from a clean target; 22 tests pass, including a fixture test that
      reads all 19 committed `.ifc` files.
- [x] **Architecture gate mutation-verified (2 distinct violations):** adding a
      geometry dep to `ifc-model`, and switching `ifc-geometry` to
      `features = ["scalar"]`, each make `no_backend_dependency.rs` FAIL;
      restoring makes it green. The gate is real, not decorative.
- [x] **Kernel boundary proven by construction:** `cargo build -p geom-kernel
      --no-default-features` compiles the contract with zero backend code.

## Stage 1 — Parser & schema

- [ ] EXPRESS `.exp` → schema table (entity, supertype chain, attribute names)
      for IFC2x3 TC1, IFC4 ADD2 TC1 **and** IFC4x3 ADD2, as data rather than
      generated code. Inputs are already local: `references/ifc-spec/`.
      Generator reads `/mnt/backup`, commits its output, so a clean checkout
      still builds.
- [ ] Handle the cross-version rename problem explicitly: `IfcBuildingElement`
      (2x3/4) vs `IfcBuiltElement` (4x3), and the 16 entities 4x3 drops.
- [ ] mmap + **record-aligned** partitioning (resync to `#<digits>=`; see the
      pitfall in `packages/ifc/AGENTS.md`) + rayon parallel scan.
- [ ] Value scanner: refs, lists, typed values, enums, strings, reals.
- [ ] `\X\`, `\X2\`, `\X4\` unicode escape decoding — not latin-1 only.
- [ ] **Validation:** entity count equals an independent raw `#<id>=` scan;
      1-partition vs N-partition totals identical; every type name in the corpus
      resolves in the schema.
- [ ] **Measure:** MiB/s and scaling p1→p20 on the largest available model.

## Stage 2 — Mesh boolean (the decisive stage)

This is what determines whether the OpenCascade-free premise holds.

- [ ] Evaluate `boolmesh` (MPL-2.0, pure Rust, glam-only) and `manifold-rust`
      against the CSG fixtures (`bath_csg_solid`,
      `issue_1155_halfspace_flyaway`,
      `issue_2019_wall_two_overlapping_openings`). **Adopting beats building**
      if one passes — check MPL-2.0 vs our MIT before vendoring.
- [ ] Implement `MeshBoolean` for `backend::scalar` (own or wrapping the above).
- [ ] **Validation:** manifold-in → manifold-out on every fixture; volume of
      `a \ b` plus volume of `a ∩ b` equals volume of `a` within tolerance
      (a triangulation-invariant check, not an index-buffer comparison).
- [ ] **Measure:** wall-minus-N-openings throughput vs IfcOpenShell on the same
      input. Publish the number, whichever way it falls.

## Stage 3 — Shape lowering

- [ ] Swept solids (extrusion, revolution), B-rep, tessellation, mapped items,
      half-space clipping, CSG trees.
- [ ] `geom-topology`: exact topology for the surfaces IFC actually uses
      (plane/cylinder/cone/sphere/torus) + `Tessellate` to `geom-mesh`. Scope
      discipline matters — a full NURBS kernel is not the goal.
- [ ] `IfcRelVoidsElement` opening cuts end-to-end through `ShapeLowerer`.
- [ ] **Validation:** every fixture lowers to a structurally valid mesh; the
      `issue_*` fixtures reproduce the behaviour their names describe.

## Stage 4 — SIMD acceleration

- [ ] AVX2 + AVX-512 paths for the wide regular passes: vertex transform,
      per-triangle AABB, broad-phase overlap, batch tri-tri intersection.
      (Not topological work — SIMD does not help there.)
- [ ] **Validation:** differential test vs `backend::scalar` on identical input, bitwise
      or within an explicit tolerance. No differential test → not trusted.
- [ ] **Measure:** speedup per pass, and end-to-end. Report honestly, including
      passes where SIMD did not help.

## Stage 5 — Properties, and the openBIM layer

Properties come first: most real IFC work is property work, and it needs no
geometry.

- [ ] `ifc-properties`: property sets, quantities, and **type→occurrence
      inheritance precedence** (occurrence wins). Unit resolution against
      `IfcUnitAssignment`, including prefixed and derived units.
- [ ] `ids`: parse buildingSMART IDS and audit a model. Validate against the IDS
      corpus in `references/ifclite/packages/ids/src/__corpus__/`, which carries
      `pass-`/`fail-` cases — an oracle we already have on disk.
- [ ] `clash`: broad phase (BVH) + narrow phase on the injected kernel.
- [ ] `bcf`: export findings so they leave this toolchain.
- [ ] **Validation:** for IDS, every `pass-` case passes and every `fail-` case
      fails, with *not applicable* distinguished from *passed* — the distinction
      that makes an audit trustworthy.

## Stage 6 — 4D/5D and diff

- [ ] `ifc-schedule` (`IfcTask`/`IfcWorkSchedule`), `ifc-cost`
      (`IfcCostItem`/`IfcCostSchedule`).
- [ ] `diff`: GUID-matched semantic diff (added/removed/moved/property-changed),
      not a text diff.

## Stage 7 — Bindings

- [ ] `bindings/python`: pyo3 + maturin, abi3 wheels. **Release the GIL around
      parse and geometry** — the structural win over `ifcopenshell-python`,
      since the Rust side is already parallel.
- [ ] `bindings/wasm`: wasm-bindgen. Requires `ifc-step` to accept `&[u8]`
      (no mmap in the browser) and no mandatory native backend.
- [ ] **Validation:** wasm bundle size published; a browser round-trip parsing a
      real model.

## Stage 8 — Publishable library

- [ ] Remove/gate `-C target-cpu=native` (see HERMES.md pitfalls) — a published
      binary must not SIGILL on older CPUs.
- [ ] `cargo doc` API reference published; `#![warn(missing_docs)]` enforced.
- [ ] Public API review: what does an application actually need?
- [ ] Benchmark suite vs IfcOpenShell on a shared corpus.

## Explicitly not planned

- **GPU mesh boolean.** Branchy, topological, precision-sensitive, and per-element
  work too small to amortize a PCIe transfer. GPU stays for large regular
  batches (broad-phase, ray casts, voxelization) behind the off-by-default
  feature. See `docs/adr/0002`.
- **Any C++ geometry dependency.** See `docs/adr/0003`.
