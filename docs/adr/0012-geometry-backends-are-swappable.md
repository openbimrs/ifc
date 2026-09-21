# 0012 — Geometry backends are swappable, whole or per area

- **Status:** Accepted
- **Date:** 2026-09-21
- **Deciders:** openbimrs contributors
- **Supersedes:** —
- **Amends:** [ADR 0004](/adr/0004-geometry-bridge-not-kernel) — compilation stays opt-in; this decides *whose* kernel runs

## Context

[ADR 0004](/adr/0004-geometry-bridge-not-kernel) established that this
crate is a bridge, not a kernel: it lowers IFC into a neutral DAG and calls
someone else's compiler. A 2026-09-14 amendment admitted mesh compilation
behind an opt-in `compile` feature.

That amendment left one thing undecided, and the implementation chose badly.
`compile_product_mesh` constructed a concrete pair inline:

```rust
let compiler = ReferenceMeshCompiler::new(BoolmeshBoolean);
```

The upstream contracts are traits — `MeshCompiler`, `MeshBoolean`,
`Tessellator`, `CurveEvaluator`, `MeshPlaneSection`, `ExactCompiler` — so the
architecture was swappable while our single call site was not. A consumer who
wanted CGAL, OCCT, or a GPU tessellator had to fork `compile.rs`. That also
made honest benchmarking impossible: comparing two backends on the same file
is exactly the measurement a bridge should make easy.

Geometry kernels are not interchangeable in practice. One is fast at mesh
booleans, another is trusted for exact B-rep, a third is the only one licensed
for a given deployment. Forcing a single choice for all areas is the wrong
shape.

## Decision

**Backend selection is a parameter, never a constant.**

1. Every compilation entry point is generic over the upstream contract, and the
   backend is borrowed so one instance serves a whole run:

   ```rust
   pub fn compile_product_mesh_with<B: MeshCompiler>(
       backend: &B, model: &Model, product: EntityId, tolerance: Tolerance,
   ) -> GeometryResult<Option<TriMesh>>
   ```

2. **Two granularities are supported, because kernels differ by area.**
   *Whole-kernel*: implement `MeshCompiler` over your own engine; none of the
   reference execution path is linked. *Per-area*: keep the reference
   traversal and replace one capability, e.g.
   `ReferenceMeshCompiler::new(MyBoolean)`.

3. **The seam and the engine are separate features.** `compile` brings the
   contracts a caller implements against. `compile-reference-backend` adds the
   reference engine and the convenience wrappers. A bring-your-own-kernel build
   enables `compile` alone and links no backend at all.

4. **A default still exists and is named.** `default_backend()` returns the
   reference pair rather than hiding it inside a function body, so a caller can
   log it beside a benchmark or compare against it explicitly.
   `compile_product_mesh` delegates to `compile_product_mesh_with`, so the
   convenience path and the explicit path cannot drift.

## Consequences

Backend choice stays application policy, which is what ADR 0004 required: the
file never determines it, and two backends given the same model must agree on
what the file means. Choosing between them is a question of speed, robustness,
or licence.

A kernel author needs no changes here — they implement a published upstream
trait. Benchmarking two kernels over one corpus is now a loop, not a fork.

The cost is one extra feature name and a generic parameter on three functions.
Callers who want the old behaviour enable `compile-reference-backend` and call
`compile_product_mesh` exactly as before.

`tests/backend_seam_linkage.rs` inspects the resolved dependency graph and
fails if the engine leaks into the `compile` feature, if the contracts go
missing from it, or if either reaches a default build.
`tests/backend_swap.rs` defines a kernel outside this crate and asserts the
mesh a caller receives is the one their own code produced.
