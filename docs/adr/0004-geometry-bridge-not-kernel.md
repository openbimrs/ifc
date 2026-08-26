# 0004 — Geometry bridge, not geometry kernel

- **Status:** Accepted
- **Date:** 2026-08-26
- **Deciders:** openbimrs contributors
- **Supersedes:** —

## Context

IFC geometry spans three resource schemas — 112 entities and 23 types across
`IfcGeometryResource`, `IfcGeometricModelResource`, and
`IfcGeometricConstraintResource`. Interpreting them requires resolving units,
nested placement chains, profile definitions, and representation reuse.

*Computing* with the result — triangulating, evaluating NURBS, executing
booleans, sectioning — is a different discipline. It is also not IFC-specific:
the same algorithms serve STEP, CityGML, and native CAD formats.

Bundling both in one crate has two costs. Every consumer of IFC geometry
semantics pulls in a numerical stack whether or not they compute. And the
algorithms become IFC-shaped, so the next format re-implements them.

## Decision

We will keep `ifc-geometry` a **bridge**: it resolves IFC meaning and lowers it
into the format-neutral Axiolid geometry DAG. It will not triangulate, evaluate
NURBS, perform booleans, or select an execution provider.

Only representation-level Axiolid crates are dependencies. Execution providers
(CPU, GPU) are excluded from this workspace's dependency graph entirely.

## Alternatives considered

| Option | Why not |
| --- | --- |
| Geometry algorithms inside `ifc-geometry` | IFC-shaped algorithms; every semantic consumer pays for a numerical stack; next format re-implements |
| Depend on OpenCascade / a C++ kernel | Contradicts the pure-Rust dependency-graph guarantee; heavy build and distribution cost |
| No geometry crate; expose raw entities | Pushes unit resolution and placement chaining onto every application, which each then gets subtly wrong |

## Consequences

**Positive**

- Geometry algorithms are written once, format-neutrally, and reused.
- A semantic consumer of IFC compiles no numerical execution code.
- The IFC side can be tested against representation output without needing an
  evaluator.
- Provider choice (CPU/GPU/parallel) is an application decision made downstream.

**Negative / costs**

- An application needing computed geometry must integrate a second library.
  "Read an IFC and get a mesh" is not a single-crate operation.
- Capabilities absent from the kernel are absent from the pipeline. Plane
  sectioning is the current example: it is a kernel concern, it does not exist
  upstream yet, so plan derivation from 3D bodies is unavailable.
- The Axiolid dependency is pinned by exact git revision, so upgrades are
  deliberate rather than automatic.

**Follow-ups / risks to watch**

- Pressure will recur to add "just one small algorithm" here. Each such addition
  re-couples IFC to computation; route them upstream instead.
- Lowering coverage is partial by design and must remain auditable — see
  [ADR 0005](/adr/0005-scaffold-modules-declare-ownership).

## Relation to existing code

- `ifc-geometry/src/lib.rs` states the scope commitment
- `ifc-geometry/src/lower/dispatch.rs` holds `IMPLEMENTED` and `PLANNED` as data
- `ifc-geometry/tests/no_backend_dependency.rs` enforces provider exclusion
- Root `Cargo.toml` pins Axiolid representation crates by revision
