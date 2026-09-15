# Scope: alignment geometry (IFC4x3 linear placement)

Status: scoping only. No implementation authorised yet.
Date: 2026-09-15

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

