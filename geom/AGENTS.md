# AGENTS.md — geom/

The shared geometry kernel. **Knows nothing about IFC.** If a type here mentions
`IfcWall`, a property set, or a GUID, it belongs in `ifc/`.

Read `docs/adr/0002` (hardware abstraction) and `docs/adr/0003` (pure-Rust
boolean) before changing anything structural here.

## Crates (low → high)

| Crate | Role | May depend on |
| --- | --- | --- |
| `geom-core` | Data only: `Vec3`, `Aabb`, `TriMesh`, `Tolerance` | nothing internal |
| `geom-kernel` | **The contract** — traits only, no algorithms | `geom-core` |
| `geom-cpu` | Portable scalar backend, the correctness oracle | core + kernel |
| `geom-simd` | SIMD backend, runtime feature detection | core + kernel |
| `geom-gpu` | Optional GPU backend, off by default | core + kernel |
| `geom-dispatch` | Runtime selection; the only crate knowing all backends | all of the above |

## Hard rules

1. **No C++ dependency, ever.** The entire premise is being lighter than
   IfcOpenShell+OpenCascade. If you believe a native library is unavoidable,
   write an ADR arguing it — do not just add it. See `docs/adr/0003`.
2. **`geom-core` is data, not algorithms.** Algorithms live in backends. A
   clever routine added to `geom-core` becomes a second kernel that nobody can
   swap out.
3. **`geom-kernel` contains no `impl` of its own traits.** It is the boundary;
   an implementation there would make the boundary a lie.
4. **Backends never depend on each other.** `geom-simd` must not call into
   `geom-cpu`. Sharing goes through `geom-core`, or the code belongs there.
5. **Scalar must stay correct without reference to anything else.** It is what
   everything else is checked against, which is why it stays simple and
   intrinsic-free.
6. **Trait methods stay coarse.** Whole meshes and batches, never a single
   triangle — dynamic dispatch per triangle would erase the optimization.

## Adding a backend

1. New crate `geom/<name>/`, depending only on `geom-core` + `geom-kernel`.
2. Implement the kernel traits and a `capabilities()` reporting availability
   **on the current machine** — return `available: false` rather than failing
   at call time.
3. Register it in `geom-dispatch::Dispatcher::detect`.
4. Add a differential test against `geom-cpu` on identical input. A backend
   without one is not trusted, regardless of how fast it is.

## Performance claims

Never assert one. `docs/ROADMAP.md` requires a measured number from a benchmark
for every performance statement, and a differential test proving the fast path
agrees with scalar.
