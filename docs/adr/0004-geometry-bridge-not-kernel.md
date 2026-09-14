# 0004 — Geometry bridge, not geometry kernel

- **Status:** Accepted
- **Date:** 2026-08-26
- **Amended:** 2026-09-14 — compilation admitted behind an opt-in feature
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

> **Amended 2026-09-14.** The second paragraph no longer holds without
> qualification: one execution provider is admitted, in `ifc-geometry` only,
> behind the non-default `compile` feature. The first paragraph stands — the
> bridge still implements no geometry. See
> [the amendment](#amendment-2026-09-14-compilation-behind-an-opt-in-feature)
> for the enforced conditions.

## Alternatives considered

| Option | Why not |
| --- | --- |
| Geometry algorithms inside `ifc-geometry` | IFC-shaped algorithms; every semantic consumer pays for a numerical stack; next format re-implements |
| Depend on OpenCascade / a C++ kernel | Contradicts the pure-Rust dependency-graph guarantee; heavy build and distribution cost |
| No geometry crate; expose raw entities | Pushes unit resolution and placement chaining onto every application, which each then gets subtly wrong |
| Sibling crate (`ifc-mesh`) owning the adapter *(weighed 2026-09-14)* | Preserves the original exclusion, but splits lowering from its only consumer across a crate boundary for a rule rather than a reason; ceremony disproportionate to a ~50-line adapter |

## Amendment (2026-09-14): compilation behind an opt-in feature

The original decision excluded execution providers from this workspace's
dependency graph **entirely**. That proved stricter than the goal required.

Measured first: `ifc-geometry` already lowers every representation-item family
it recognises (`PLANNED` is empty), and the Axiolid mesh compiler already
evaluates the ten solid families that lowering emits. The two sides were
compatible in practice while remaining unconnected, so "read an IFC and get a
mesh" was impossible for reasons that no longer had a technical cause.

**What changed.** `ifc-geometry` may depend on the mesh-compile provider, under
three conditions that are enforced rather than documented:

1. Optional, and reachable only through the non-default `compile` feature. The
   check walks the feature graph from `default`, so a feature merely *named*
   `compile` that some default feature enables still fails.
2. `ifc-geometry` only. No other IFC crate may name an execution provider.
3. The default and `--no-default-features` columns link zero provider crates.

**What did not change.** The bridge still implements no geometry: no
triangulation, no NURBS evaluation, no boolean execution. `compile` calls a
compiler that already exists and translates its refusal into IFC terms.

**Why a feature rather than a sibling crate.** A sibling crate was the
ADR-preserving option and was considered. It was rejected as ceremony
disproportionate to a ~50-line adapter, and because the alternative splits
lowering from its only consumer across a crate boundary for a rule, not a
reason. The cost accepted: feature unification can enable `compile` for an
entire dependency graph from one consumer. Bounded by condition 3 — the
provider is absent unless something explicitly asks.

**Policy that stays in the application.** Provider choice and memory budget are
arguments, never defaults invented by the bridge. Tolerance is the exception:
lowering converts every length to metres, so the bridge is the one participant
that knows the file's true scale.

## Consequences

**Positive**

- Geometry algorithms are written once, format-neutrally, and reused.
- A semantic consumer of IFC compiles no numerical execution code.
- The IFC side can be tested against representation output without needing an
  evaluator.
- Provider choice (CPU/GPU/parallel) is an application decision made downstream.

**Negative / costs**

- An application needing computed geometry must opt in explicitly. Since the
  amendment above, "read an IFC and get a mesh" is a single-crate operation
  behind `--features compile`, but it is never the default.
- Capabilities absent from the kernel are absent from the pipeline. Plane
  sectioning is the current example: it is a kernel concern, it does not exist
  upstream yet, so plan derivation from 3D bodies is unavailable.
- The Axiolid dependency is pinned by exact git tag, so upgrades are
  deliberate rather than automatic.

**Follow-ups / risks to watch**

- Pressure will recur to add "just one small algorithm" here. Each such addition
  re-couples IFC to computation; route them upstream instead.
- Lowering coverage is partial by design and must remain auditable — see
  [ADR 0005](/adr/0005-scaffold-modules-declare-ownership).

## Relation to existing code

- `ifc-geometry/src/lib.rs` states the scope commitment
- `ifc-geometry/src/lower/dispatch.rs` holds `IMPLEMENTED` and `PLANNED` as data
- `ifc-geometry/src/compile.rs` is the opt-in seam; it calls a compiler and
  translates refusals, and implements no geometry itself
- `ifc-geometry/tests/no_backend_dependency.rs` keeps every other IFC crate free
  of provider dependencies
- `ifc-model/tests/package_architecture.rs` enforces the three amendment
  conditions, including that `compile` stays unreachable from `default`
- `ifc-geometry/tests/kernel_free_build.rs` checks the resolved graph per feature
  column, so the "zero provider crates by default" claim is measured
- `ifc-geometry/tests/compile_pairing.rs` keeps lowering and compilation paired:
  each family yields a mesh or a typed refusal that names its entity
- Root `Cargo.toml` pins Axiolid crates by exact git tag (`v0.1.8`)
