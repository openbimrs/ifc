# AGENTS.md — packages/geometry/

The shared geometry kernel. **Knows nothing about IFC.** If a type here mentions
`IfcWall`, a property set, or a GUID, it belongs in `packages/ifc/`.

Read `docs/adr/0002` (hardware abstraction), `0003` (pure-Rust boolean, the
decision the project rests on) and `0005` (why these crates exist).

## Crates, low → high

| Crate | Role | Depends on |
| --- | --- | --- |
| `geom-core` | scalars, tolerance, `Vec3`/`Mat4`, `Aabb` | — (root) |
| `geom-mesh` | `TriMesh`: the exchange currency | core |
| `geom-profile` | 2D cross-sections, polygons, triangulation | core |
| `geom-curve` | line/arc/NURBS evaluation, trimming, arc length | core |
| `geom-surface` | plane/cylinder/cone/sphere/torus, NURBS patches | core, curve |
| `geom-sweep` | extrude, revolve, sweep along directrix | core, profile, curve, mesh |
| `geom-topology` | exact B-rep: vertex→edge→loop→face→shell→solid | core, curve, surface |
| `geom-tessellate` | exact geometry → triangles, chord tolerance | core, mesh, curve, surface, topology |
| `geom-spatial` | BVH / octree / grid + queries | core, mesh |
| `geom-measure` | area, volume, centroid, inertia | core, mesh, profile |
| `geom-kernel` | **the trait contract** + hardware backends | core, mesh |

Sizing evidence (IFC4 entity counts): 23 `IfcProfileDef` subtypes, 36 curve
entities, 37 surface entities, 11 swept-solid forms, ~37 topology entities. Each
crate above corresponds to a real cluster in the standard, not an invented one.

## Tiers (enforced, not advisory)

```text
  L0  math / data      geom-core
   ^
  L1  representation   geom-mesh, geom-profile, geom-curve, geom-surface,
                       geom-topology
   ^
  L2  algorithms       geom-sweep, geom-tessellate, geom-spatial, geom-measure,
                       geom-kernel
```

Dependencies point **down or sideways, never up**. Same-tier edges are fine —
`geom-surface` needs `geom-curve` to trim. The edge that matters is the L1 rule:
representation types stay usable without dragging in an algorithm crate, which
is what lets a foreign kernel accept our `TriMesh` without accepting our kernel.

`geom-core.tests/layering.rs` enforces all of this, plus "no crate here depends
on `packages/ifc/`". A new crate with no entry in its `TIERS` table fails the
build — declaring the layer is part of creating the crate. The gate is
mutation-verified by `scripts/probe_layering_gate.sh` (5 probes: reversed seam,
tier inversion, root gaining a sibling, untiered crate, commented-out decoy).

## The two swaps (do not break these)

1. **Whole-kernel swap.** `packages/ifc/` depends on `geom-kernel` *traits*,
   never on a backend implementation. A better kernel implements the traits and
   the IFC layer gains it with no call-site change.
2. **Hardware-backend swap.** `geom-kernel::backend::{scalar,simd,gpu}` are
   cargo features, selected at runtime by `backend::Dispatcher`. Adding AVX-512
   or a GPU path = adding an impl, never editing callers.

`scalar` is the **correctness oracle**: it must compile everywhere and take no
`target_feature`. Every other backend is validated *against* it by differential
test. A backend without a differential test is not trusted.

## Rules

- **`geom-core` gains no dependencies.** It is the shared vocabulary; if it
  depended on a sibling, the siblings would stop being siblings.
- **No rendering types anywhere.** No colour, material, or presentation flag —
  that is `ifc-style`'s job. A kernel with `getColorBuffer()` on its base type
  cannot be refactored without touching a renderer.
- **No serialization derives.** Persisting geometry belongs to a codec layer;
  a derived `Serialize` on a kernel type freezes its layout forever.
- **Tolerance is a parameter, never a global.** BIM arrives in millimetres *and*
  metres; a file-scope `1e-9` is wrong in one of them.
- **Dirty geometry is a state, not an error.** Non-manifold input is normal;
  represent it, report it, do not panic on it.
- **Coarse trait granularity.** Backends receive whole meshes and batches, never
  one triangle, so dynamic dispatch is amortised.

## Scope discipline (the thing that kills geometry kernels)

`geom-curve`, `geom-surface` and `geom-topology` are where a NURBS effort
balloons into a multi-year CAD project. Target **what real IFC files contain**:
planes and cylinders dominate, extrusion covers most solids, NURBS appears at
the margin. Curve/curve intersection and general surface interrogation stay out
until a fixture demands them.

## Status — read before assuming a capability exists

Scaffold. Every crate here is documentation + intent; `geom-tessellate` carries
the `Tessellate` trait and `ChordTolerance`, `geom-kernel` carries the traits and
backend detection, and `backend::scalar`'s boolean deliberately returns
`Unsupported` rather than a wrong mesh. Check the module doc before assuming
behaviour is implemented.
