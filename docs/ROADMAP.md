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

- [x] `geom/` + `ifc/` package split; 10 crates across the two groups.
- [x] `geom-kernel` trait contract (`MeshBoolean`, `Capabilities`, `GeomError`).
- [x] Three backend crates (scalar/SIMD/GPU) + `geom-dispatch` runtime selection.
- [x] SIMD runtime feature detection (`is_x86_feature_detected!`), no
      compile-time hardware lock-in.
- [x] `ifc-shape` seam generic over `K: MeshBoolean`; proven backend-agnostic by
      a test injecting a kernel from neither backend crate.
- [x] ADRs 0001 (split), 0002 (hardware abstraction), 0003 (pure-Rust boolean).
- [x] **Validated:** `cargo build/test/clippy -D warnings/fmt --check` all
      green; 22 tests pass, including a fixture test that reads all 19
      committed `.ifc` files.
- [x] **Architecture gate mutation-verified:** adding `geom-cpu` to
      `ifc/model/Cargo.toml` makes `no_backend_dependency.rs` FAIL (both tests),
      and removing it restores green. The gate is real, not decorative.

## Stage 1 — Parser & schema

- [ ] EXPRESS `.exp` → schema table (entity, supertype chain, attribute names)
      for IFC2X3 + IFC4, as data rather than generated code.
- [ ] mmap + **record-aligned** partitioning (resync to `#<digits>=`; see the
      pitfall in `ifc/AGENTS.md`) + rayon parallel scan.
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
- [ ] Implement `MeshBoolean` for `geom-cpu` (own or wrapping the above).
- [ ] **Validation:** manifold-in → manifold-out on every fixture; volume of
      `a \ b` plus volume of `a ∩ b` equals volume of `a` within tolerance
      (a triangulation-invariant check, not an index-buffer comparison).
- [ ] **Measure:** wall-minus-N-openings throughput vs IfcOpenShell on the same
      input. Publish the number, whichever way it falls.

## Stage 3 — Shape lowering

- [ ] Swept solids (extrusion, revolution), B-rep, tessellation, mapped items,
      half-space clipping, CSG trees.
- [ ] `IfcRelVoidsElement` opening cuts end-to-end through `ShapeLowerer`.
- [ ] **Validation:** every fixture lowers to a structurally valid mesh; the
      `issue_*` fixtures reproduce the behaviour their names describe.

## Stage 4 — SIMD acceleration

- [ ] AVX2 + AVX-512 paths for the wide regular passes: vertex transform,
      per-triangle AABB, broad-phase overlap, batch tri-tri intersection.
      (Not topological work — SIMD does not help there.)
- [ ] **Validation:** differential test vs `geom-cpu` on identical input, bitwise
      or within an explicit tolerance. No differential test → not trusted.
- [ ] **Measure:** speedup per pass, and end-to-end. Report honestly, including
      passes where SIMD did not help.

## Stage 5 — Publishable library

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
