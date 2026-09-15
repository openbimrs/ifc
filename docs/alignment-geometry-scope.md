# Scope: alignment geometry (IFC4x3 linear placement)

Status: B (authoring) LANDED in f87aa87. A and C updated below.
Date: 2026-09-15, revised after axiolid/kernel#105 was fixed.

## What already works

Measured, not assumed. ifc-alignment is further along than a
function count suggests:

| Capability | State |
| --- | --- |
| Horizontal segment lowering | done, `lower_horizontal_segment` |
| Horizontal layout assembly | done, `lower_horizontal_layout` |
| Partial/continuity-aware assembly | done, `lower_horizontal_layout_partial` |
| Vertical segment lowering | done, `lower_vertical_segment` |
| Cant evaluation + layout | done, `cant_at`, `CantLayout` |
| Linear placement resolution | done, `resolve_linear_placement` |
| Station equations | done, `station_equations` |

The crate already depends on axiolid-core, axiolid-curve and
axiolid-model. It is a geometry bridge, like ifc-geometry.

Correction to an earlier claim: it is NOT true that alignment
is unwired from geometry. It lowers to the neutral DAG directly.
What is missing is narrower and is stated below.

## Gap 1: no 3D composition (the blocker)

Horizontal lowers to a 2D curve. Vertical lowers to its own
curve. Nothing composes them into the 3D centreline that
IfcGradientCurve denotes.

This is a KERNEL capability gap, not an IFC one. In the pinned
kernel (tag v0.1.8):

```
Curve2 { Line, Circle, Ellipse, Polyline, BSpline, Intrinsic }
Curve3 { Line, Circle, Ellipse, Polyline, BSpline }
```

`Curve2::Intrinsic` carries curvature-as-a-function-of-arc-length,
which is what represents a clothoid EXACTLY. `Curve3` has no
Intrinsic variant.

So a spiralled alignment with a vertical profile has no exact 3D
representation in the kernel today. Composing one would mean
either approximating the spiral as a BSpline3 -- which violates
the alignment invariant that transition intent is preserved
exactly -- or extending the kernel.

This is an upstream Axiolid decision, matching ADR 0004:
capabilities absent from the kernel are absent from the pipeline.

## Gap 2: IFC4x3 linear-geometry entities absent from ifc-geometry

Probed by name against ifc-geometry/src:

| Entity | Present |
| --- | --- |
| IfcCurveSegment | yes |
| IfcGradientCurve | no |
| IfcSegmentedReferenceCurve | no |
| IfcOffsetCurveByDistances | no |
| IfcAxis2PlacementLinear | no |
| IfcLinearPlacement | no |
| IfcPointByDistanceExpression | no |

Six of seven absent. This matters because a product placed by
IfcLinearPlacement cannot be positioned by ifc-geometry at all:
`product_world_transform` resolves IfcLocalPlacement chains only.

A road sign or sleeper placed along an alignment therefore has
no world transform, independent of gap 1.

## Proposed order

Each step is independently shippable and gated.

**Step A -- linear placement resolution (no kernel change).**
Teach ifc-geometry IfcLinearPlacement, IfcAxis2PlacementLinear and
IfcPointByDistanceExpression by delegating distance-along
resolution to the existing ifc-alignment code. Unblocks
positioning products along alignments. Largest practical win,
no upstream dependency.

Boundary question to settle first: ifc-geometry must not depend
on ifc-alignment (sibling rule, package_architecture.rs). Options:
invert -- ifc-alignment implements placement resolution and
ifc-geometry exposes a hook; or move the shared distance-along
contract into the generic layer. Needs a decision, likely an ADR.

Verified: package_architecture.rs asserts a domain crate may only
reach GENERIC, reporting that a cross-layer edge must "compose
sibling capabilities in the facade/application instead". So the
inversion or generic-layer contract is required, not optional.

**Step B -- authoring (kernel-free, mirrors ADR 0011).**
Author IfcAlignment, the H/V/C layouts and their segments as
records, inside ifc-alignment, indexing the reader slot modules.
Independent of gaps 1 and 2: authoring segment records needs no
3D composition. This is the direct parity item.

**Step C -- 3D composition (blocked upstream).**
Needs a kernel decision on exact 3D transition curves. Options:
add Curve3::Intrinsic; or a composed gradient relation pairing a
2D intrinsic plan with a vertical profile. Until then,
IfcGradientCurve should be a TYPED REFUSAL, matching how
VIENNESEBEND is already handled -- not a BSpline approximation.

## Recommendation

Do B first: unblocked, kernel-free, closes the stated parity gap.
Then A, once the boundary question has an ADR. Treat C as an
upstream Axiolid issue and keep refusing exactly until it lands.

Explicitly NOT recommended: approximating spirals in 3D to make
alignment look complete. It would violate the crate invariant and
silently degrade survey-grade data.


## Revision: C is resolved upstream

axiolid/kernel#105 was closed as completed. `Curve3::Elevated(Elevated3)`
now pairs a boxed `Curve2` plan with an `ElevationLaw`, so an exact spiral
composes with an exact vertical profile without approximating either.

Two findings in that fix matter to this crate:

- `Intrinsic2` previously had no evaluator at all, so evaluating a point on
  a clothoid was unavailable in 2D as well. `arc_length.rs` now provides it.
- Height is a function of PLAN distance, not 3D arc length. The two diverge
  by sqrt(1 + g^2) wherever grade is non-zero. The convention is named in
  the type, so this crate must not re-derive it.

A B-spline plan is refused upstream: its parameter is not arc length.

Not yet consumable here: the fix is on main and in no tag. ADR 0004 pins
axiolid by exact tag, so lowering to `Curve3::Elevated` waits for a release.

## Revision: B is done

Landed in f87aa87. Eight authoring functions, 5 tests, 8/8 mutations
killed. Readers and authoring now share `src/slot.rs`.

Vertical arcs remain unlowerable (`push_constant_gradient` refuses
anything but CONSTANTGRADIENT). That is this crate\x27s own limit, not the
kernel\x27s, and is unrelated to #105.

## Remaining: A (linear placement)

Unchanged and still blocked on a boundary decision, not on the kernel.
IfcLinearPlacement, IfcAxis2PlacementLinear, IfcPointByDistanceExpression,
IfcGradientCurve, IfcSegmentedReferenceCurve and IfcOffsetCurveByDistances
are all absent from ifc-geometry, so a product placed along an alignment
still has no world transform.

package_architecture.rs refuses ifc-geometry -> ifc-alignment. Either
invert the dependency or lift the distance-along contract into the generic
layer. That is an ADR, not a patch.
