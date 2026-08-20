# IFC package implementation plan

Status: active architecture scaffold
Last updated: 2026-08-19

This file is implementation state, not ambient instructions. Read
`AGENTS.md` first. Read this plan only for roadmap work or to locate a blocked
cross-crate dependency. Detailed work lives in each crate's `PLAN.md`.

## Goal

Build an IFC stack whose generic record model, codecs, domain projections,
geometry adapters, and application workflows remain independently evolvable.
The scaffold exists so focused agents can implement one bounded slice without
re-deriving package architecture or creating monoliths.

## Architecture baseline

- `ifc-model` owns the schema-agnostic entity graph and `Codec` port.
- STEP and XML are working codec adapters; JSON is not implemented.
- Domain crates are borrowed projections over `ifc-model`; implementation depth
  varies and is recorded in crate plans.
- Geometry-facing adapters emit exact, neutral geometry values/DAGs and cannot
  select a backend.
- IFC resources that mix roles are split by attribute projection, per ADR 0008.
- Cross-domain operations are application services outside leaf domain crates.

## Work queue

### Integration order

Implement in dependency order. A later wave may proceed early only when it can
use an existing stable contract and does not invent the missing lower layer.

### Wave 0 - architecture scaffold and executable boundaries

- [x] `IFC-000` - IFC root progressive context and cross-crate plan.
- [x] `IFC-001` - add every crate's `AGENTS.md` and `PLAN.md`.
- [x] `IFC-002` - add local context/plans at complex geometry-facing module boundaries.
- [x] `IFC-003` - compile every scaffold module and remove stale orphan source files.
- [x] `IFC-004` - add and mutation-verify dependency/context/orphan gates.
- [x] `IFC-005` - verify the frozen candidate from a clean isolated worktree.

### Wave 1 - model capabilities shared by all projections

Task state is owned only by `ifc-model/PLAN.md`; this package plan records order:

1. `MODEL-INV` - reverse-reference index for EXPRESS inverse attributes.
2. `MODEL-MUT` - explicit mutation transaction/builder for authoring.
3. `MODEL-TRV` - budgeted traversal and reusable relationship queries.
4. `MODEL-PRV` - optional source/provenance side tables without domain knowledge.

These are ports, not reasons for the model to learn material, geometry, or
quantity semantics.

### Wave 2 - complete the IFC to neutral geometry bridge

Task state is owned only by `ifc-geometry/PLAN.md`; this is the enforced order:

1. `GEOM-CONTRACT` - classify validated unit directions versus magnitude vectors.
2. `GEOM-SESSION` - establish one builder/memo/provenance session per output DAG.
3. `GEOM-INPUT`, `GEOM-CTX`, and `GEOM-PLACE` - own source slots and frames once.
4. `GEOM-PROFILE`, `GEOM-CURVE`, and `GEOM-SURFACE` - emit exact neutral nodes.
5. `GEOM-BREP`, `GEOM-TESS`, `GEOM-SOLID`, and `GEOM-MAP` - compose representations.
6. `GEOM-CENSUS` - report represented, interpreted, and lowered coverage separately.

Alignment and georeferencing keep task state in their own crate plans.

### Wave 3 - domain projections that touch geometry workflows

- [ ] `IFC-MATERIAL` - material identity/profile/layer semantics; see `ifc-material/PLAN.md`.
- [ ] `IFC-PROPERTIES` - authored properties, quantities, units, and mutation ports;
      see `ifc-properties/PLAN.md`.
- [ ] `IFC-STYLE` - presentation projections with representation-item references only;
      see `ifc-style/PLAN.md`.
- [ ] `IFC-GEOREF` - project-to-map coordinate operation; see `ifc-georef/PLAN.md`.
- [ ] `IFC-ALIGNMENT` - alignment parameters and exact neutral curve output;
      see `ifc-alignment/PLAN.md`.

The authoring quantity flow is deliberately L4 orchestration:

```text
ifc-properties read view -> geometry compiler/kernel measure
                          -> ifc-properties mutation port
```

Neither leaf crate depends on the other.

### Wave 4 - remaining domain projections

`ifc-classification`, `ifc-resource`, `ifc-schedule`, `ifc-structural`,
`ifc-systems`, and `ifc-cost` retain independent plans and crate boundaries.
They are intentional domains, not candidates for automatic consolidation.

### Wave 5 - validation and codecs

- [ ] `IFC-VALIDATE` - schema-aware validator with bounded rule execution;
      see `ifc-validate/PLAN.md`.
- [ ] `IFC-JSON` - codec after the shared scalar/typed-value contract is explicit;
      see `ifc/PLAN.md` for the future crate decision.
- [ ] `IFC-STEP-PERF` - parallel STEP parsing only after correctness/benchmark baselines;
      see `ifc-step/PLAN.md`.

## Cross-resource ownership decisions

| IFC concept | Geometry projection | Semantic projection | Orchestration |
| --- | --- | --- | --- |
| `IfcProfileDef` | exact shape in `ifc-geometry` | name/type only when needed | app chooses use |
| `IfcMaterialProfile*` | profile ref, cardinal point, offsets | material/name/priority/category in `ifc-material` | app associates product |
| `IfcMaterialLayerSetUsage` | direction/offset geometry input | layer identity/composition in `ifc-material` | app chooses representation |
| `IfcElementQuantity` | none | authored values/read-write in `ifc-properties` | app computes and populates |
| geometric representation context | local project frame/precision in `ifc-geometry` | none | app composes map transform |
| map conversion/projected CRS | none | geodetic metadata + transform in `ifc-georef` | app applies at export/query boundary |
| presentation appearance | none | all style semantics in `ifc-style` | renderer joins by entity ID |

## Plan update protocol

A task owner:

1. claims one unchecked task ID in the nearest `PLAN.md`;
2. records prerequisites or a blocker before broadening scope;
3. implements only the files named by that slice, splitting sooner if ownership
   becomes mixed;
4. runs the listed exit evidence and records the exact command/result;
5. checks the item off and adds newly discovered work as a new ID, never as an
   anonymous TODO in source or `AGENTS.md`.

Plans may record status, blockers, evidence, and decisions. They must not contain
secrets, transient process IDs, or unverifiable performance claims.

## Risks and current critiques

- The parallel geometry refactor has already replaced the IFC-local primitive
  vocabulary with exact neutral `axiolid-*` values. Any remaining docs claiming
  `ifc-geometry` defines or depends on a kernel trait are stale and must be
  corrected rather than preserved for compatibility.
- The neutral-DAG seam is formalized by ADR 0009. The metadata-backed package
  architecture gate is authoritative; local manifest scans are smoke tests only.
- `ifc-geometry::lower::Tolerance` remains in an exact-profile API but is unused.
  Before public stabilization, remove it or give it a real non-tessellation
  semantic; do not carry a compatibility parameter in a pre-1.0 API by habit.
- Cross-resource views can duplicate slot interpretation. Geometry-affecting
  material/profile/context slots need named IFC-side input modules and tests,
  while semantic crates keep their own semantic projections.
- Empty public modules would create false API promises. Scaffold children stay
  crate-private until implemented.
- Shared master contains concurrent geometry edits. Never stage or rewrite those
  paths opportunistically; reconcile from the latest HEAD before integration.

## Completion log

- `IFC-000` through `IFC-003` - the frozen candidate owns 18 crate pairs and 55
  progressive boundaries plus 187 correctly owned compiled private module paths.
  The owned IFC test matrix and warning-denied rustdoc pass.
- `IFC-004` - dependency/context/reachability/file-map/API-surface tests pass;
  fifteen deliberate invalid mutations were caught and restored byte-for-byte.
  A custom Cargo binary target using `#[path]` was accepted and restored.
- `IFC-005` - rebased isolated candidate on base `3ff7231` passed
  `./scripts/gate.sh` including workspace/all-feature tests, clippy, docs,
  feature matrices, and isolated crate builds.

## Package-wide exit evidence

```bash
cargo fmt --all -- --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
scripts/gate.sh
```

Performance work additionally requires a committed benchmark definition,
baseline environment, and measured comparison. A green correctness gate is not
a performance claim.
