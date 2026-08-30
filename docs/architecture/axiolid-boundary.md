# The Axiolid boundary

`ifc-geometry` is a **bridge**, not a geometry engine. This page states exactly
where the responsibility transfers, because misunderstanding this boundary is
the most common way to over-estimate what the crate can do.

## The split

```text
  ifc-model              ifc-geometry                  Axiolid
  (untyped graph)  -->   typed views                   (neutral kernel)
                         + IFC resolution        -->   GeometryGraph
                         + unit handling               + providers
                         + placement chains
```

**`ifc-geometry` answers:** what does this IFC entity *mean* geometrically?
Which profile, which placement, which units, which representation, which
sub-shape does an `IfcMappedItem` reuse?

**Axiolid answers:** what do I compute with that meaning?

`ifc-geometry`'s own crate documentation is explicit:

> It does not triangulate, evaluate NURBS, perform booleans, or select
> execution providers.

That is a design commitment, not a temporary limitation. Geometry algorithms
belong to a format-neutral kernel so they are not re-implemented per file
format, and so the IFC crate never grows a dependency on a CPU or GPU provider.
`ifc-geometry/tests/no_backend_dependency.rs` enforces the second half.

## Consequences you must plan for

**Curve representation is not curve evaluation.** `ifc-geometry` reads an
`IfcBSplineCurveWithKnots` into a neutral B-spline description. Producing points
along it is an Axiolid concern. If you need discretised geometry — for drawing,
export, or measurement — that call is downstream of this crate.

**Booleans are represented, not executed.** `IfcBooleanResult` lowers into a DAG
node describing the operation and its operands. Evaluating it requires a mesh
Boolean provider.

**Sectioning is not available.** Deriving a 2D plan by cutting 3D bodies at a
height requires a plane/solid section operation. That operation does not exist
in `ifc-geometry` — correctly, since it is a kernel concern — and, as of the
pinned revision, **there is no plane-section or slice operation in Axiolid
either**. Applications needing plan derivation from 3D must currently source
that capability elsewhere.

**Axiolid is a contract kernel more than an algorithm library.** Much of it
defines neutral vocabulary and validated representations; a smaller portion is
executable algorithms. Check Axiolid's own capability documentation rather than
assuming an operation exists because a crate named after it does.

## What Axiolid does provide

Verified at the revision pinned by this workspace:

- Exact and filtered geometric predicates (`orient2d`, `incircle`, `insphere`)
  with degeneracy tests — a correctness oracle.
- Polygon ring utilities: signed area and orientation.
- Ear-clipping triangulation for hole-free rings, and an Earcut-backed
  triangulation provider.
- Neutral values for primitives, profiles, curves, surfaces, and B-rep topology.
- Scalar reference evaluation for polynomial and rational B-spline curves and
  surfaces, including analytic first surface partials.
- An immutable shared geometry DAG with typed IDs.
- A mesh Boolean provider, bounded by its own mesh contract.
- CPU context and a GPU seam — a seam, not a bundled kernel suite.

## The pin

The workspace pins Axiolid crates to an exact git revision rather than a
version range, so geometry behaviour is reproducible across builds:

```toml
axiolid-core = { git = "https://github.com/axiolid/axiolid-kernel.git", rev = "c136b61…" }
```

Production lowering pins only representation-level crates — `core`, `mesh`,
`model`, `primitive`, `profile`, `curve`, `surface`, and `topology`.
`axiolid-scalar` is a dev-only oracle used by import regressions; it does not
enter the published adapter's production dependency graph.
