# Changelog

All notable changes to **nehirde** are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

One entry per change under `## [Unreleased]` as you land work; cut a version
section on release.

## [Unreleased]

### Added
- **`geom/` + `ifc/` package architecture.** Ten crates across two package
  groups: `geom/{core,kernel,cpu,simd,gpu,dispatch}` and
  `ifc/{schema,parser,model,shape}`. `geom/` is an IFC-agnostic shared geometry
  kernel; `ifc/` is pure IFC logic.
- **Swappable geometry kernel.** `geom-kernel` holds traits only
  (`MeshBoolean`, `Capabilities`, `GeomError`); `ifc/` depends on the contract
  and never on a backend, so the geometry implementation can be replaced without
  touching the IFC layer. Enforced by `ifc/shape/tests/no_backend_dependency.rs`,
  which reads the manifests and fails the build on violation — mutation-verified
  to actually fail when a backend dependency is added.
- **Hardware abstraction.** Scalar (`geom-cpu`, the correctness oracle), SIMD
  (`geom-simd`, runtime `is_x86_feature_detected!` for AVX2/AVX-512), and
  optional GPU (`geom-gpu`, off by default) backends behind one contract, with
  `geom-dispatch` selecting the most specialized available backend at runtime.
- ADR 0001 (geom/ifc split + kernel contract), ADR 0002 (hardware abstraction),
  ADR 0003 (pure-Rust mesh boolean instead of OpenCascade).
- Repo scaffold: `docs/` (roadmap, ADRs, this changelog), `references/`
  symlinks to IfcOpenShell + ifc-lite clones on `/mnt/backup/`,
  `test/fixtures/` with 19 edge-case `.ifc` files pulled from those two repos,
  `target` symlinked to `/mnt/backup/build-cache/` (sparse root disk),
  progressive `AGENTS.md` context files.

### Notes
- No C++ geometry dependency anywhere in the graph — the premise of the project.
  `geom-cpu`'s boolean currently returns `Unsupported` and reports
  `mesh_boolean: false` rather than emitting a wrong mesh; a real implementation
  is Stage 2 in `docs/ROADMAP.md`.
