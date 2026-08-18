# 0001 — Split the workspace into `geom/` and `ifc/`, with a trait contract between them

- **Status:** Accepted
- **Date:** 2026-08-18
- **Deciders:** GeneralPawz, Hermes
- **Supersedes:** —

## Context

The goal is the best IFC library in Rust — a lightweight, high-performance
alternative to IfcOpenShell, which is powerful but couples IFC handling to
OpenCascade, a very heavy C++ geometry dependency. Applications should be able
to build on top of our library without inheriting that weight.

Two forces shape the layout:

1. **Geometry is not IFC-specific.** CSG, meshing, and spatial queries are
   useful to other formats too. A shared kernel should not know what an
   `IfcWall` is.
2. **Geometry kernels are replaceable, and ours will not be the last word.**
   A robust boolean kernel is a large effort. We must be able to swap in a
   better one — ours or a third party's — without rewriting the IFC layer.

A constraint learned from the sibling `solibri-rs` workspace: its `geometry`
crate pulls `manifold3d` for 3D CSG, a native C++ library measured at ~256 MB
of its debug build directory, inherited by every crate depending on it. That
is precisely the failure mode we are avoiding, and it arrived through an
ordinary dependency edge — so the boundary needs enforcement, not etiquette.

## Decision

Two top-level package directories, each containing crates:

```text
geom/   core, kernel, cpu, simd, gpu, dispatch
ifc/    schema, parser, model, shape
```

`geom/kernel` holds **traits only** — the contract. `ifc/` depends on
`geom-kernel` (traits) and `geom-core` (data types) and **never** on a backend
crate. `ifc/shape` is the sole crate in `ifc/` that touches geometry at all;
`schema`, `parser`, and `model` have no geometry dependency whatsoever.

The concrete backend is selected by the **application** at the top and injected
into `ifc-shape`. Neither the IFC layer nor the kernel contract names a backend.

## Alternatives considered

| Option | Why not |
| --- | --- |
| One flat `crates/` dir | Does not express the geom/ifc split the project is organized around, and gives no natural home for per-backend crates. |
| `ifc` depends on a `geom` facade crate | A facade still binds one implementation; swapping means editing the facade's dependencies, and the facade tends to accrete IFC-shaped helpers. |
| `#[cfg]` feature flags to pick a backend | Bakes one hardware choice in at build time and makes cross-backend differential testing impossible — you cannot compare scalar vs SIMD output if only one is compiled. |
| Generic parameter on every IFC type | Viral: `Wall<K>` infects every signature in the workspace for a choice that matters only at the geometry seam. |

## Consequences

**Positive**

- The geometry kernel is genuinely swappable; the requirement is structural.
- Consumers doing property audits or quantity takeoffs compile no geometry at
  all — a capability the IfcOpenShell+OpenCascade stack does not offer.
- Backends can all be built at once and differentially tested against scalar.

**Negative / costs**

- More crates to navigate than a flat layout.
- Dynamic dispatch costs an indirect call where `&dyn` is used. Mitigated by
  keeping trait granularity **coarse** (whole meshes and batches, never a single
  triangle), so the call is amortized across thousands of elements.
- The contract must be designed before the implementation, which is harder than
  letting an API emerge.

**Follow-ups / risks to watch**

- If `ifc-shape` starts needing kernel capabilities that are awkward to express
  as traits, revisit the contract rather than reaching for a backend directly.
- Watch that `geom-core` stays data-only; algorithms leaking into it would make
  it a second, unswappable kernel.

## Relation to existing code

- `geom/kernel/src/lib.rs` — the contract and its rationale.
- `ifc/shape/src/lib.rs` — the seam; generic over `K: MeshBoolean`.
- `ifc/shape/tests/no_backend_dependency.rs` — **enforces** this ADR by reading
  the `ifc/*` manifests and failing the build if a backend dependency appears.
  Verified to fail when violated, not merely to pass today.
