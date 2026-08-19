# Changelog

All notable changes to **nehirde** are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

One entry per change under `## [Unreleased]` as you land work; cut a version
section on release.

## [Unreleased]

### Added
- **ifcXML codec (`ifc-xml`)** implementing the same `Codec` trait as
  `ifc-step`, proving serialization is genuinely pluggable: the model crate did
  not change to accommodate it. Schema-aware attribute naming is optional, with
  a positional fallback so files from unknown schemas still round-trip.
- **EXPRESS schema parser (`ifc-schema`)** reading the official `.exp` files.
  Verified against all three shipped schemas: IFC2x3 TC1 (653 entities),
  IFC4 ADD2 TC1 (776 entities, 397 types), IFC4x3 ADD2 (876 entities).
  Provides `is_a` subtype queries and STEP-positional attribute names.
- **`ifc` facade features** for codecs (`step`, `ifcxml`), `schema`, and each
  domain, plus `codecs` / `domains` / `full` bundles. `default = ["step"]`.
- **`Codec::detect`** (defaulting to `false`) so `ifc::read_path` selects a
  codec by content sniffing, then extension.
- **Layered geometry package scaffold** with one immutable neutral DAG,
  typed-handle B-rep topology, exact curve/surface/profile values, narrow
  operation-provider traits, separate CPU execution/GPU adapter crates, and a
  feature-gated `geom` facade. Core-only resolves 3 unique packages; default
  resolves 5, including the facade itself.
- **Authoritative IFC geometry support ledger** covering all 163 IFC4 ADD2 TC1
  declarations: 112 entities (23 abstract, 89 concrete), 13 selects, seven
  enums, three defined types, and 28 functions. Coverage and ownership gates are
  mutation-verified.
- Costing fixture `test/fixtures/costing/costing_schedule.ifc` with cost
  schedules, items, values, quantities, property sets, and an entity type from
  no IFC schema.

### Changed
- Active `ifc-geometry` lowering uses the canonical profile, primitive, CSG, and
  backend-neutral `geom-model` vocabulary. The pre-DAG request values remain
  deprecated source-compatibility shims and are rejected from active lowering.
- Local `target-cpu=native` flags are scoped to x86_64 so AArch64 cross-checks do
  not inherit invalid x86 features.

### Fixed
- ifcXML wrote numeric-looking strings (`IfcApplication.Version = "0.1"`) as
  plain XML attributes, so re-reading inferred `Real(0.1)` and silently changed
  the value's kind. Such strings now become typed child elements.

### Added
- **Working STEP codec and entity model.** `ifc-model` holds the entity graph
  (`Value`, `Entity`, `Model`, GUID codec, type index, dangling-reference
  detection); `ifc-step` implements lexer, parser and writer over it. All 19
  committed fixtures parse (7,920 entities across IFC2x3, IFC4 and IFC4X3_ADD2)
  and round-trip structurally intact.
- **`Codec` trait in `ifc-model`**, so serialization is pluggable: STEP today,
  ifcXML and IFC-JSON as future crates implementing the same trait. The model
  depends on no codec.
- **`ifc` facade crate** exposing every domain as a cargo feature. A thin
  (`step`) build resolves 26 crates; `full` resolves 51, and the thin build
  links no geometry kernel and no `glam`.
- **`ifc-cost` as the worked example of a domain view** — borrows `&Model`,
  owns no storage, therefore optional at compile time with no data loss.
- `ifc-cli`: `info` and `types` commands over real files.
- ADR 0006 recording the model/domain and model/serialization separations.

### Fixed
- Two architectural tests found to be passing vacuously, caught by mutation
  testing: the feature-gating test never checked the `default` feature set, and
  nothing checked `Model::insert` id reuse (dropping its guard duplicated the
  entity in export order while all tests stayed green).
- An earlier dependency test read `cargo metadata`'s package list, which lists
  every workspace member regardless of features; it now reads `cargo tree`.

### Added
- **Official IFC schemas as reference material** (`references/ifc-spec/`, 249 MB
  on `/mnt/backup`, symlinked, never committed): EXPRESS schemas for IFC2x3 TC1,
  IFC4 ADD2 TC1 and IFC4x3 ADD2, the IFC4 ifcXML `.xsd`, 737 property-set
  definition XMLs, and the full IFC4 HTML documentation. Documented in
  `references/AGENTS-ifc-spec.md`, including the browser-User-Agent requirement
  that otherwise 403s every download.
- **8 geometry crates**, sized from the schema rather than guessed:
  `geom-profile` (23 `IfcProfileDef` subtypes), `geom-curve` (36 curve
  entities), `geom-surface` (37 surface entities), `geom-sweep` (11 swept-solid
  forms), `geom-topology` (~37 topology entities), `geom-tessellate`,
  `geom-spatial`, `geom-measure`.
- **9 IFC crates**, each backed by an entity count: `ifc-style` (48),
  `ifc-structural` (39), `ifc-systems` (23), `ifc-material` (22),
  `ifc-resource` (21), `ifc-classification` (12), `ifc-georef` (8),
  `ifc-alignment` (IFC4x3 linear referencing), `ifc-validate` (47 schema
  functions + 2 global rules).
- ADR 0005 recording the spec-driven expansion and the evidence behind it.

### Changed
- `geom-brep` renamed to `geom-topology` (the standard's own vocabulary); its
  `Tessellate` trait and `ChordTolerance` moved to `geom-tessellate`, tests
  intact.
- **The architecture gate is now an allowlist.** "Only `ifc-geometry` may touch
  geometry" was too narrow once `ifc-georef` and `ifc-alignment` existed —
  alignment geometry is deliberately not part of the building-shape pipeline.
  `MAY_USE_GEOMETRY` names the three permitted crates; a fourth requires editing
  the list and saying why. Three tests, each mutation-verified: non-allowlisted
  crate gaining geometry, allowlisted crate enabling a backend feature, and the
  allowlist naming a crate that does not exist.

### Deferred
- **WASM bindings.** `bindings/wasm` → `bindings/_deferred-wasm`, removed from
  workspace members and added to `exclude`, so it is not built, tested or
  linted. Kept rather than deleted because its constraints (no threads, no
  `is_x86_feature_detected!`, size budget) are an argument for the runtime
  backend selection already in `geom-kernel`.

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
