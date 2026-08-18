# AGENTS.md — packages/ifc/

Pure IFC logic. Read `docs/adr/0001` (the split) and `0004` (layout + how the
boundary is now enforced) before changing dependencies.

## Crates

| Crate | Role | Geometry? |
| --- | --- | --- |
| `ifc-schema` | Schema as data: entity table, supertype chain | none |
| `ifc-step` | STEP/IFC-SPF reader: mmap, partition, parallel scan | none |
| `ifc-model` | Indexed semantic views: type buckets, spatial tree | none |
| `ifc-properties` | Property sets, quantities, unit resolution | none |
| `ifc-cost` | `IfcCostItem` / `IfcCostSchedule` (5D) | none |
| `ifc-schedule` | `IfcTask` / `IfcWorkSchedule` (4D) | none |
| `ifc-geometry` | Lowers representation items to meshes | **contract only** |

## The rule

**Only `ifc-geometry` may touch geometry**, and only through `geom-kernel`
traits with `default-features = false` (plus `geom-core`/`geom-mesh` data). It
never names a backend.

Two consequences worth protecting:

- The geometry kernel is swappable — a better one implements the same traits.
- **A consumer doing property/quantity/COBie work compiles no geometry code at
  all.** That is a concrete advantage over IfcOpenShell, where the geometry
  engine is not optional in practice. `ifc-properties` having zero geometry deps
  is the point, not an accident.

Both are enforced by `ifc-geometry/tests/no_backend_dependency.rs`, which reads
these manifests and fails the build. It is mutation-verified: adding a geometry
dep to `ifc-model`, or changing `ifc-geometry` to `features = ["scalar"]`, each
makes it fail.

If you need geometry in `ifc-step` or `ifc-model`, the design is wrong — that
work belongs in `ifc-geometry`.

## Dependency direction

`packages/ifc/` may depend on `packages/geometry/`. It must **never** depend on
`packages/openbim/`, `bindings/`, or `apps/`. IDS/BCF/clash are consumers of this
layer.

## Fixtures

Tests read `test/fixtures/` via `env!("CARGO_MANIFEST_DIR")` joined to
`../../../test/fixtures` (three levels: `packages/ifc/<crate>`). Do not hardcode
absolute paths. See `test/fixtures/AGENTS.md` for what each file exercises.

## Status

Scaffold. `ifc-step` recognizes STEP headers and schema tokens; the parser
itself, and everything in `ifc-properties`/`ifc-cost`/`ifc-schedule`, is
reserved. See `docs/ROADMAP.md`.
