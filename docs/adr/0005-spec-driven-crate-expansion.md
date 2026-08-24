# 0005 — Spec-driven crate expansion

- **Status:** Accepted
- **Date:** 2026-08-18
- **Deciders:** GeneralPawz, Hermes
- **Supersedes:** extends 0001 and 0004

## Context

The crate list up to now was invented from intuition. We now hold the official
EXPRESS schemas (`references/ifc-spec/`, see that dir's `AGENTS.md`), so the
decomposition can be driven by what the standard actually contains rather than
by guesswork.

Measured from the schemas themselves:

| Schema | Entities | Types |
| --- | --- | --- |
| IFC2x3 TC1 | 653 | — |
| IFC4 ADD2 TC1 | 776 | 397 |
| IFC4x3 ADD2 | 876 | — |

IFC4x3 adds **116 entities** over IFC4 and removes 16. Two findings drove this
ADR:

1. **Geometry is far larger than "meshes and booleans."** IFC4 carries 23
   `IfcProfileDef` subtypes, 36 curve entities, 37 surface entities, 6
   NURBS/B-spline entities, 11 swept-solid forms and ~37 topology entities.
   A single `axiolid-brep` crate covering "B-rep" was hiding four distinct
   subsystems, each with its own algorithms and failure modes.

2. **Whole IFC concept areas had no home.** Counting IFC4 entities per area
   with no dedicated crate: presentation/style 48, structural analysis 39,
   topology 37, ports/distribution systems 23, materials 22, profiles 23,
   resources 21, classification/document 12, actor/address 10,
   constraint/approval 9, georeferencing 8.

## Decision

Expand both package groups so each substantial subsystem is its own crate, and
rename where the spec's own vocabulary is clearer than our invention.

### `../axiolid/`

| Crate | Role | Why separate |
| --- | --- | --- |
| `axiolid-core` | scalars, tolerance, vectors, transforms, AABB | root of the graph |
| `axiolid-profile` | 2D profiles, polygons, triangulation, 2D boolean | 23 `IfcProfileDef` subtypes; purely 2D, testable in isolation |
| `axiolid-curve` | lines, arcs, composite curves, NURBS curves, trimming | 36 curve entities; evaluation/parameterisation is its own domain |
| `axiolid-surface` | plane/cylinder/cone/sphere/torus, NURBS surfaces | 37 surface entities; needed before B-rep means anything |
| `axiolid-mesh` | `TriMesh`, validation, repair, topology queries | the exchange currency between everything |
| `axiolid-sweep` | extrude, revolve, sweep along directrix, loft | 11 swept forms; consumes profile+curve, produces mesh/brep |
| `axiolid-topology` | vertex/edge/loop/face/shell/solid, half-edge | ~37 topology entities; the exact-representation counterpart to `axiolid-mesh` |
| `axiolid-tessellate` | curve/surface/B-rep → triangles, chord tolerance | one place where "how fine?" is decided |
| `axiolid-spatial` | BVH, octree, grid, nearest/ray queries | acceleration, generic over payload; every query crate needs it |
| `axiolid-measure` | area, volume, centroid, inertia, bounds | quantity takeoff needs these without booleans |
| `axiolid-kernel` | the trait contract + hardware backends | unchanged role: the swap boundary |

`axiolid-brep` is renamed `axiolid-topology` (the spec's word, and it says what the
crate holds) and its surface/curve concerns move to the crates above.

### `packages/`

Kept: `ifc-schema`, `ifc-step`, `ifc-model`, `ifc-geometry`, `ifc-properties`,
`ifc-cost`, `ifc-schedule`.

Added, each backed by an entity count above: `ifc-material`, `ifc-style`,
`ifc-structural`, `ifc-resource`, `ifc-classification`, `ifc-georef`,
`ifc-systems`, `ifc-alignment`, `ifc-validate`.

`ifc-alignment` exists because IFC4x3's 14 alignment entities plus its spiral
curve types (`IfcClothoid`, `IfcCosineSpiral`) are the entire reason 4x3 exists;
they are civil-infrastructure geometry that a building-only consumer should not
have to compile.

## Alternatives considered

| Option | Why not |
| --- | --- |
| Keep the small crate list, add modules instead | Modules inside one crate cannot be depended on selectively. A quantity-takeoff tool would compile NURBS evaluation to read a property set. |
| One crate per IFC domain (~30) | Over-decomposition. Domains like "shared building services" share no algorithms; the split would be nominal, not structural. |
| Generate crates from the schema | The schema describes data, not algorithm boundaries. `IfcProfileDef` and `IfcSurface` are unrelated as code even though both are "geometry resources." |

## Consequences

**Positive**

- Selective compilation is real: a property-set reader depends on
  `ifc-schema` + `ifc-step` + `ifc-properties` and compiles no geometry at all.
- Each crate has a natural, small validation corpus, so a stage can be
  "done" against evidence rather than by assertion.
- The 4x3 rename `IfcBuildingElement` → `IfcBuiltElement` confirms ADR 0001's
  schema-as-data choice: three schema versions differ in entity *names*, so
  generated per-version types would triple the surface area.

**Negative / costs**

- 20 more crates to keep compiling, documented, and non-circular.
- Most are reserved scaffolding today. A reserved crate that never gains an
  implementation is dead weight, and the roadmap must actually retire or fill
  each one.

**Follow-ups / risks to watch**

- `axiolid-curve`/`axiolid-surface` are where a NURBS effort would balloon. Scope is
  deliberately "what real IFC files contain," not a general CAD kernel.
- Crate count now exceeds the useful limit for a flat dependency table; the
  per-group `AGENTS.md` files carry the graph instead.

## Relation to existing code

`../axiolid/*`, `packages/*`, workspace `Cargo.toml` members and
`[workspace.dependencies]`, and the architecture gate in
`packages/ifc-geometry/tests/no_backend_dependency.rs`, which now checks
every crate in both groups.
