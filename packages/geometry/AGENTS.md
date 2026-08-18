# AGENTS.md — packages/geometry/

The shared geometry kernel. **Knows nothing about IFC.** If a type here mentions
`IfcWall`, a property set, or a GUID, it belongs in `packages/ifc/`.

Read `docs/adr/0002` (hardware abstraction), `0003` (pure-Rust boolean), and
`0004` (why backends are features, not crates).

## Crates

| Crate | Role | Depends on |
| --- | --- | --- |
| `geom-core` | Data only: `Vec3`, `Mat4`, `Aabb`, `Tolerance` | — (graph root) |
| `geom-mesh` | `TriMesh`: the discrete form every backend consumes | `geom-core` |
| `geom-brep` | Exact topology + `Tessellate` bridge to `geom-mesh` | `geom-core`, `geom-mesh` |
| `geom-kernel` | The trait CONTRACT + `backend::{scalar,simd,gpu}` | `geom-core`, `geom-mesh` |

Add a new crate here only if it is genuinely format-agnostic. If it needs to
know what an `IfcWall` is, it belongs in `packages/ifc/`.

## The contract/implementation split (the thing to not break)

`geom-kernel` carries both the traits and the backends, separated by features:

```toml
default = ["scalar", "simd"]   # scalar is the correctness oracle
gpu     = []                   # OFF: pulls a driver stack
```

- **Consumers (libraries) take `default-features = false`** — traits only, zero
  backend code compiled. `packages/ifc/*` and `packages/openbim/clash` do this,
  and `ifc-geometry/tests/no_backend_dependency.rs` fails the build if one
  forgets.
- **Applications opt in** — `apps/ifc-cli` sets `features = ["scalar", "simd"]`.

🚨 **`default-features = false` on a workspace dependency is IGNORED unless the
root `[workspace.dependencies]` entry also sets it.** Cargo only warns. The root
entry sets it; if you ever move `geom-kernel` to a plain `{ path = ... }` there,
every consumer silently gains the backends and the boundary becomes cosmetic.

## Rules

1. **No rendering types.** No colour, material, or presentation flag.
2. **No serialization derives.** Persisting geometry belongs to a codec layer;
   a `Serialize` derive here freezes the layout forever.
3. **Tolerance is a parameter, never a global.** Models arrive in millimetres
   *and* metres.
4. **Backends never diverge silently.** A new backend must be differential-tested
   against `backend::scalar` on the same input. That is the entire reason the
   design uses traits rather than `#[cfg]` — preserve the ability to build
   several backends at once.
5. **A backend that cannot do something reports `false` in `Capabilities` and
   returns `GeomError::Unsupported`.** Never emit a wrong mesh. A corrupt solid
   propagates into every downstream area, volume, and clash result.

## Status

Scaffold. `backend::scalar`'s boolean returns `Unsupported`; `geom-brep` has the
`Tessellate` trait and no topology types yet. `docs/ROADMAP.md` Stage 2 (boolean)
and Stage 4 (B-rep) are the real work.
