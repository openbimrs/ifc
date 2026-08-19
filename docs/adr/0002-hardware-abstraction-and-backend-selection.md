# 0002 — Hardware abstraction: runtime backend selection, scalar as oracle

- **Status:** Superseded by [0009](0009-layered-geometry-dag.md)
- **Date:** 2026-08-18
- **Deciders:** GeneralPawz, Hermes
- **Supersedes:** —

> **Amended by [0004](0004-package-layout-and-backend-features.md).** The
> reasoning below stands. The backends are no longer separate crates: they are
> `geom-kernel`'s `backend::{scalar,simd,gpu}` modules behind cargo features,
> and `geom-dispatch` is now `geom_kernel::backend::Dispatcher`.
>
> **Topology and oracle ownership superseded by
> [0012](0012-scalar-reference-ownership.md).** The crate names in this ADR
> (`geom-cpu`, `geom-simd`, `geom-gpu`, `geom-dispatch`) and every path in
> "Relation to existing code" are historical: none of them exist. The real
> crates are `geom-backend-cpu` (an execution *context*, explicitly **not** the
> correctness oracle) and `geom-backend-gpu`. The scalar reference is owned by
> `geom-scalar` per 0012. The reasoning below -- runtime selection, single
> portable binary, differential validation against a scalar reference -- stands.


## Context

Performance is a first-class goal: we intend to beat the IfcOpenShell stack on
the same inputs, and the dev box (Xeon w7-3565X) has AVX-512 and AMX. But a
library others build on cannot assume that hardware. The workspace currently
compiles with `-C target-cpu=native`, which is right for a machine we control
and produces a binary that SIGILLs on an older CPU.

We also want GPU acceleration *where it genuinely helps*, without making every
consumer compile a GPU stack to read a wall.

## Decision

Hardware specialization is a **runtime** choice behind the `geom-kernel` traits,
not a compile-time `#[cfg]` choice.

- One crate per execution strategy: `geom-cpu` (scalar), `geom-simd`,
  `geom-gpu`. Each reports a `Capabilities` struct describing what it can do
  **on the current machine**.
- `geom-dispatch` probes all of them at startup and selects the most
  specialized backend that is both available and implements the requested
  operation.
- `geom-cpu` is the **correctness oracle**: portable, no intrinsics, always
  available. Every other backend is validated by differential test against it.
  <!-- Superseded by 0012: the oracle is `geom-scalar`; `geom-backend-cpu` is an
  execution context and explicitly not the oracle. The principle stands. -->
- SIMD uses `is_x86_feature_detected!` + `#[target_feature]`, so a single
  portable binary still uses AVX-512 where present.
- GPU is behind an off-by-default `gpu` feature and carries a work-size
  threshold (`gpu_threshold_triangles`), so the "is it worth the PCIe trip"
  judgement is enforced by the dispatcher rather than left to a comment.

A backend reporting `available: false` is never selected.

## Alternatives considered

| Option | Why not |
| --- | --- |
| `-C target-cpu=native` only | Fast on the dev box, crashes elsewhere. Unacceptable for a library. |
| `#[cfg(target_feature)]` per backend | One backend per binary; cross-backend differential testing becomes impossible, which is exactly how we intend to prove SIMD correctness. |
| Always-on GPU dependency | Reproduces the OpenCascade weight problem we exist to solve. |
| GPU boolean | Mesh CSG is branchy, topological, and precision-sensitive; a wall-minus-two-openings cut is far too small to amortize a transfer. Not planned. |

## Consequences

**Positive**

- Single portable binary that still exploits AVX-512 where available.
- Every optimized path has a reference implementation to be checked against, so
  a performance claim can be backed by a differential test plus a measurement.
- GPU stays optional; the default dependency footprint stays small.

**Negative / costs**

- Every operation must be written at least twice (scalar + optimized) to benefit.
- Capability plumbing is overhead that a single-target library would not pay.
- Runtime detection costs a startup probe (negligible, done once).

**Follow-ups / risks to watch**

- The workspace `.cargo/config.toml` still sets `target-cpu=native`. That must
  be removed or made opt-in before anything is published; the SIMD crate's
  runtime detection is the intended mechanism. Tracked in `docs/ROADMAP.md`.
- Resist adding fine-grained (per-triangle) trait methods — dynamic dispatch
  there would erase the SIMD win.

## Relation to existing code

- `geom/kernel/src/capability.rs` — `Backend`, `Capabilities`.
- `geom/simd/src/lib.rs` — `SimdBackend::detect`, `SimdLevel`.
- `geom/gpu/src/lib.rs` — opt-in feature, threshold, absent-GPU-is-normal.
- `geom/dispatch/src/lib.rs` — selection policy; tested to return `None` rather
  than a backend that would fail at call time.
