# Changelog

All notable changes to **nehirde** are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

One entry per change under `## [Unreleased]` as you land work; cut a version
section on release.

## [Unreleased]

### Changed
- **Restructured into role-grouped packages.** `geom/` and `ifc/` became
  `packages/geometry/` and `packages/ifc/`, joined by `packages/openbim/`
  (`ids`, `bcf`, `clash`, `diff`), `bindings/` (`python`, `wasm`), and `apps/`
  (`ifc-cli`). Dependency direction is one-way:
  `geometry → ifc → openbim → {bindings, apps}`. 17 crates total.
- **Backends are now cargo features of `geom-kernel`, not separate crates.**
  `geom-cpu`/`geom-simd`/`geom-gpu`/`geom-dispatch` became
  `geom_kernel::backend::{scalar,simd,gpu,Dispatcher}` behind features
  `scalar` + `simd` (default) and `gpu` (off). The swap boundary is now expressed
  as a feature constraint: `packages/ifc/*` take `default-features = false`,
  applications opt in. See ADR 0004.
- **`TriMesh` moved from `geom-core` to the new `geom-mesh` crate**; `geom-core`
  is now data-and-tolerance only.
- Renamed `ifc-parser` → `ifc-step`, `ifc-shape` → `ifc-geometry`.

### Fixed
- **`default-features = false` was being silently ignored**, which would have
  made the kernel swap boundary cosmetic. Cargo drops it on a member dependency
  unless the root `[workspace.dependencies]` entry also sets it — it only emits a
  warning. Fixed at the workspace entry; applications now opt in explicitly. The
  architecture test covers this case.

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
- `geom-brep` — reserved crate for exact topology, with the `Tessellate` bridge
  to `geom-mesh`. This is the capability OpenCascade provides to IfcOpenShell;
  scope is deliberately limited to the surfaces IFC actually uses.
- `apps/ifc-cli` — working binary. `ifc capabilities` reports detected backends
  and the selected boolean implementation (currently: none, honestly).
- `packages/ifc/{ifc-properties,ifc-cost,ifc-schedule}` and
  `packages/openbim/{ids,bcf,clash,diff}` — reserved, documented crates.
- ADR 0001 (geom/ifc split + kernel contract), ADR 0002 (hardware abstraction),
  ADR 0003 (pure-Rust mesh boolean instead of OpenCascade), ADR 0004 (package
  layout + backends as features).
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
