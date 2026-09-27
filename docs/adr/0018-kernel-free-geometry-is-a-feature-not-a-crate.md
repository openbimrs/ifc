# 0018 — Kernel-free geometry is a feature, not a crate

- **Status:** Accepted
- **Date:** 2026-09-27
- **Deciders:** openbimrs contributors
- **Supersedes:** —

## Context

`ifc-geometry` serves two kinds of consumer. A drawing or quantity tool needs
representation contexts, plan and body selection, placements, units and the
typed resource views: slot reading over `ifc-model`. A viewer or kernel needs
those same views lowered into the format-neutral Axiolid DAG. Only the second
needs the `axiolid-*` representation crates, and linking them costs the first
consumer a geometry kernel it never calls.

The split between the two halves has to live somewhere. The obvious candidate
is a second crate (an `ifc-geometry-lower` beside a kernel-free
`ifc-geometry`). The alternative is one crate with an optional,
default-on `lowering` feature that gates the neutral dependencies.

This decision was made when the feature was introduced and recorded only in
the crate's context file. It is written down here so it survives that file.

## Decision

We will keep lowering inside `ifc-geometry`, behind the default-on `lowering`
feature, rather than in a separate crate.

- `lower` is a leaf module: nothing else in the crate depends on it, so the
  feature boundary holds structurally. Outside `lower` and the `compile`
  layer built on it, only feature-gated conversions such as
  `Transform::to_geom` and the `IfcBooleanOperator` conversion name an
  `axiolid-*` type; everything else reads `ifc-model` slots and compiles with
  the feature off.
- `tests/kernel_free_build.rs` checks the resolved dependency graph under
  `--no-default-features`, so a stray unconditional `use axiolid_*` fails the
  gate instead of silently relinking the kernel for 2D consumers.
- Placement resolution (`constraint::placement`) stays outside `lower`,
  because world coordinates are needed with `lowering` off.
- The `compile` features (ADR 0004, ADR 0012) sit on top of `lowering`; they do
  not change this split.

## Alternatives considered

| Option | Why not |
| --- | --- |
| A separate lowering crate | Costs a published crate name, its own release and changelog, and a manifest edit for every consumer that wants lowering, to enforce a boundary one test already enforces. |
| One crate with unconditional kernel dependencies | Every drawing and quantity consumer would link the geometry kernel. |
| Kernel-free crate by convention only | An optional dependency can be re-enabled through a feature edge; only the resolver sees it, so the boundary needs the dependency-graph test either way. |

## Consequences

**Positive**

- One crate name, one version, one changelog; `ifc-geometry` with default
  features keeps working unchanged for existing consumers.
- The kernel-free build is proven by the resolver, not by review.

**Negative / costs**

- `--all-features` cannot see a break that exists only with the feature off,
  so the gate builds, tests, lints and documents `--no-default-features`
  separately.
- Intra-doc links to feature-gated items need that separate rustdoc run.

**Follow-ups / risks to watch**

- Revisit if the kernel-free half gains heavy dependencies of its own, or
  needs a release cadence independent of lowering. Either would make a
  separate crate pay for itself.

## Relation to existing code

- `crates/ifc-geometry/Cargo.toml`: the `lowering`, `compile` and
  `compile-reference-backend` features.
- `crates/ifc-geometry/src/lib.rs`: `#[cfg(feature = "lowering")] pub mod lower`.
- `crates/ifc-geometry/tests/kernel_free_build.rs`: the dependency-graph proof.
- `scripts/gate.sh`, section `features`: the kernel-free column.
