# Roadmap

**Mission:** the best IFC library in Rust — a lightweight, high-performance
alternative to IfcOpenShell, without OpenCascade.

Validation-gated stages. No stage is "done" on a claim. Every stage lands with
(a) a cross-check against `test/fixtures/` (or a larger oracle corpus) and
(b) a measured wall-clock. Performance claims are always backed by a benchmark
number, never asserted.

Hardware on the dev box: Intel Xeon w7-3565X, 20 vCPU, AVX-512
(f/dq/bw/vl/vbmi/ifma/cd) + AMX (tile/int8/bf16), 62 GB RAM; NVIDIA RTX
4000 Ada Generation, compute capability 8.9, 20 GB VRAM. These are benchmark
targets, never the portable compile baseline.

## Stage 0 - Architecture scaffold DONE

- [x] Downward-only geometry graph: core, representation, algorithm/contract,
      backend, facade, then external format adapters.
- [x] `geom-model` immutable typed-handle DAG for exact values, mapped instances,
      CSG, sweeps, B-rep, and tessellated geometry; forward references rejected.
- [x] `geom-kernel` contains opt-in operation contracts only.
      `geom-backend-cpu` is a runtime execution context and
      `geom-backend-gpu` holds operation-specific adapters.
- [x] `geom` facade exposes lean `mesh`/`cpu` defaults plus additive
      `discrete`, `parametric`, `advanced`, `parallel`, `simd`, `gpu`, and
      `full` bundles; every important combination is built in isolation.
- [x] Runtime x86/AArch64 feature detection, optional local Rayon pool, no
      compile-time host lock-in. GPU APIs plug in through operation-specific
      seams such as `GpuGraphExecutor`.
- [x] `ifc-geometry` resolves units/placements into neutral profile and DAG
      values; it owns no duplicate primitive/kernel vocabulary and imports no
      concrete backend.
- [x] Authoritative IFC4 geometry-resource manifest: 112 entities + 23 types +
      28 functions = 163 declarations. Every declaration has explicit bridge
      and neutral ownership; scaffold status remains distinct from executable
      implementation status.
- [x] Progressive `AGENTS.md` plus non-ambient `PLAN.md` at every geometry crate.
- [x] ADRs 0001-0009.
- [x] Architecture, source-vocabulary, orphan-module, standard-trait, feature,
      and declaration gates. The dependency, source-vocabulary, and declaration
      gates were each mutation-verified to fail on a real injected violation.

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

- [ ] Port/adopt the proven 2D coplanar profile subtraction seam first; keep
      validation area-based rather than tied to one triangulation.
- [ ] Evaluate `boolmesh` (MPL-2.0, pure Rust, glam-only) and pure-Rust
      alternatives against the CSG fixtures (`bath_csg_solid`,
      `issue_1155_halfspace_flyaway`,
      `issue_2019_wall_two_overlapping_openings`). Adopting beats building if a
      candidate passes license, robustness, and dependency-weight gates.
- [ ] Implement a portable CPU `MeshBoolean` provider that composes the CPU
      execution context, retains scalar behavior as oracle, and accepts many
      opening tools in one call.
- [ ] **Validation:** manifold-in -> manifold-out on every fixture; volume of
      `a \ b` plus volume of `a intersection b` equals volume of `a` within
      tolerance (a triangulation-invariant check, not an index-buffer match).
- [ ] **Measure:** wall-minus-N-openings throughput vs IfcOpenShell on the same
      input. Publish the number, whichever way it falls.

## Stage 3 - Exact shape lowering and evaluation

- [ ] Close the 163-declaration ledger by implementation status, not just owner:
      point/placement -> curves -> profiles/sweeps -> surfaces -> topology/B-rep
      -> CSG/half-space -> tessellated sets -> derived/validation functions.
- [ ] Implement curve families: line/conics, polyline/indexed polycurve,
      composite/trim/offset, polynomial and rational B-spline.
- [ ] Implement analytic and B-spline surfaces, bounded/trimmed/swept/offset
      surface relationships, and shared-edge discretization.
- [ ] Implement swept solids (extrusion, revolution, directrix and fixed-reference
      sweep, swept disk, tapered and sectioned solids), B-rep, mapped instances,
      half-space clipping, and CSG DAG compilation.
- [ ] `IfcRelVoidsElement` opening cuts end to end through neutral DAG lowering
      and a selected backend, without `ifc-geometry` importing that backend.
- [ ] **Validation:** differential oracle fixtures from IfcOpenShell, schema
      function unit tests, invariant mesh checks, and cross-process determinism.
      Every unsupported declaration must return structured `Unsupported` rather
      than approximate or panic.

## Stage 4 - Measured hardware acceleration

- [ ] Portable CPU implementations first; then AVX2/AVX-512 paths for wide,
      regular passes: vertex transforms, per-triangle AABBs, broad phase, and
      batched triangle tests. Keep branchy topology scalar.
- [ ] Add AArch64 NEON implementations under the same operation contracts and
      exercise them in cross-target CI plus real ARM hardware when available.
- [ ] Add one concrete GPU operation executor only for measured batch-friendly
      workloads (broad phase, ray batches, voxelization, large
      transforms/tessellation). Validate requested precision in that operation;
      fall back explicitly when the device cannot satisfy it.
- [ ] **Validation:** differential tests against portable CPU plus cross-process
      determinism. No differential test means the optimized path is disabled.
- [ ] **Measure:** startup/transfer threshold, per-pass speedup, memory, and end
      to end latency. Report regressions as well as wins.

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
