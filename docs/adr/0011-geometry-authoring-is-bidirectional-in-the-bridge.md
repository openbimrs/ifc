# 0011 — Geometry authoring is bidirectional inside the bridge

- **Status:** Accepted
- **Date:** 2026-09-15
- **Deciders:** openbimrs contributors
- **Supersedes:** —

## Context

Six of the eight authoring domains now have write paths. Geometry is the
largest remaining gap: `ifc-geometry` exposes 0 authoring functions across
104 files and 31,550 lines.

Two questions had to be answered before writing any of it.

**Where does geometry authoring live?** [ADR 0007](/adr/0007-authoring-is-a-schema-layer-not-a-model-layer)
placed generic authoring in its own L2 crate, `ifc-author`. Read literally as
precedent it suggests a sibling `ifc-geometry-author`. But the reason behind
0007 does not transfer: `ifc-model` sits at L0 and `ifc-schema` at L1, so
typed setters inside `ifc-model` would have inverted a tier edge.
`ifc-geometry` already depends on both. The constraint that forced a separate
crate there is absent here.

**Can geometry authoring be kernel-agnostic?** The stated goal is authoring
with CGAL, OCCT, or any kernel -- not only Axiolid.

## Evidence

A throwaway crate was built outside this workspace, depending on
`ifc-geometry` with `default-features = false`, and used to author a real
`IfcExtrudedAreaSolid` wall body: point, direction, axis placement,
rectangle profile, extrusion. It compiled, and `ifc-geometry` read the
result back. External authoring is therefore *possible*. The spike measured
what it costs.

| Measure | Value |
| --- | ---: |
| Slot constants an authoring API for bodies needs | 201 |
| Of those unreachable from outside the crate | 183 (91%) |
| Private `mod slot` modules | 21 |
| Public `mod slot` modules | 2 |

The largest single block is `lower/profile.rs`: 73 constants across 725
lines and 26 functions. It is not a lookup table. It carries judgement --
that `IfcProfileDef` is instantiable but is a profile *label* rather than a
section and so is a permanent typed refusal; that closing an open curve
would fabricate a face the file never described; that every index was read
from IFC4 ADD2 TC1 rather than inferred. A separate crate re-derives that
reasoning or silently diverges from it.

The decisive finding was not the count. `RepresentationPurpose` -- Body,
Axis, FootPrint -- is the identifier authoring **must** agree with, because
writing `"Body"` when the reader selects on something else produces geometry
no consumer finds. It lives in `pub(crate) mod input`, re-exported only
through `pub mod lower`, which is gated behind the `lowering` feature:

```
--no-default-features : error: cannot find `lower` in `ifc_geometry`
                        note: found an item that was configured out
--features lowering   : ok
```

A kernel-free authoring crate cannot see it. The proposal defeats itself:
either link the kernel -- abandoning the property that made a separate crate
attractive -- or duplicate the identifiers, which is exactly the drift that
breaks round-trips.

Two things did reach externally and are genuine shared surface:
`units::resolve` with `UnitScale::length`, and `resource::topology::slot`.
Together, 18 of 201 constants.

## Decision

We will author geometry **inside `ifc-geometry`**, in a new `authoring`
module that sits on the IFC side of the existing `lowering` feature line.

The crate becomes bidirectional. The crate's **AGENTS.md** previously stated a
one-directional purpose (lower IFC into a DAG); it is amended to state both
directions, with the invariants below holding for each.

**Authoring links no geometry kernel.** This is what makes CGAL, OCCT, or
any other kernel a usable source. IFC geometry entities are exact
*descriptions* -- `IfcExtrudedAreaSolid`, `IfcBSplineSurface`,
`IfcBooleanResult` -- not evaluations, so writing them needs no evaluator.
The authoring API therefore takes plain `f64`, `[f64; 3]`, and index
buffers. No signature names an Axiolid type, and the module compiles with
`--no-default-features`. A kernel becomes a *source of numbers*, never a
dependency of the writer. This mirrors IfcOpenShell, whose Python authoring
API imports no OCC at all -- OCC lives on its read path.

**Kernel-computed results stay unevaluated.** Authoring a cut stores an
`IfcBooleanResult`, preserving exact intent. This keeps the standing
invariant: never tessellate in this adapter.

**Slot knowledge has one definition.** The private `mod slot` modules become
`pub(crate)` and authoring indexes them. Authoring does not restate a slot
number that lowering already owns.

**What does not change.** The bridge still implements no geometry: no
triangulation, no NURBS evaluation, no boolean execution. [ADR
0004](/adr/0004-geometry-bridge-not-kernel) stands in full, including its
three enforced conditions on the `compile` feature. Adding a write direction
does not admit computation in either direction.

## Alternatives considered

| Option | Why not |
| --- | --- |
| Separate `ifc-geometry-author` crate | Measured: 183 of 201 needed slot constants unreachable; `RepresentationPurpose` invisible without linking the kernel the split exists to avoid |
| Kernel-pluggable authoring (trait over CGAL/OCCT/Axiolid) | Solves a problem that does not exist. IFC geometry entities are descriptions; writing them needs no evaluator. An abstraction over kernels would be inert in every authoring signature |
| Route geometry through `ifc-author` generic builder | No new crate, but no geometry-specific validation either: no unit conversion, no direction normalization, no profile rules |
| Leave `ifc-geometry` read-only, author elsewhere later | Leaves the largest domain at zero indefinitely and defers the same decision |

## Consequences

**Positive**

- Slot layouts, unit conversion, and representation identifiers have one
  definition serving both directions. A round-trip test can catch drift
  because both sides are compiled together.
- Authoring is kernel-agnostic by construction, not by abstraction: no
  authoring signature can name a kernel type.
- A semantic or 2D consumer still links no kernel. Authoring is available in
  the `--no-default-features` build, which lowering is not.

**Negative / costs**

- The largest crate in the workspace grows further. This argues for a later
  split by *resource family*, not by direction -- direction is the seam that
  maximises duplication.
- Two error vocabularies share one enum: "this file states something I
  cannot represent" and "you asked me to write something invalid".
- The feature graph gains a third axis. `kernel_free_build.rs` must prove
  authoring stays kernel-free rather than assuming it.

**Follow-ups / risks to watch**

- Pressure will recur to compute during authoring ("just evaluate this
  boolean before writing it"). Route it upstream, as ADR 0004 requires.
- Alignment authoring depends on this module and on a lowering path that
  does not yet exist; it stays blocked until then. Deriving plans from
  solids remains an upstream kernel concern, not an IFC one.

## Relation to existing code

- `ifc-geometry/src/authoring/`: the write direction. Kernel-free; compiles
  with `--no-default-features`.
- `ifc-geometry/src/{resource,curve,surface,solid}/*.rs`: `mod slot` becomes
  `pub(crate)` so authoring indexes one definition.
- `ifc-geometry/src/units.rs`: already kernel-free; authoring writes lengths
  through the same scale lowering reads.
- `ifc-geometry/tests/kernel_free_build.rs`: extended to prove the authoring
  path links no kernel.
- The **AGENTS.md** of `ifc-geometry`: purpose amended from one-directional to
  bidirectional.
- `ifc-model/tests/package_architecture.rs`: unchanged. `ifc-geometry`
  remains the single `COMPILE_HOST`, and authoring adds no dependency.
