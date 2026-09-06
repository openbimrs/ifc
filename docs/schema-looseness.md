# Schema looseness

Some IFC files are schema-valid and still carry no recoverable geometric
intent: the EXPRESS declaration permits a combination the geometry cannot
honour, and no `WHERE` rule rejects it. The file is not corrupt and the
authoring tool is not necessarily at fault -- the schema is simply looser
than the geometry it describes.

This page lists the cases this library refuses for that reason, so you can
tell them apart from features that are merely unimplemented.

## Why we refuse rather than cope

Each case below has an obvious "just be lenient" reading, and every one of
them silently produces wrong geometry rather than no geometry. A wrong curve
renders fine, passes a viewer, and misleads whoever measures it. A typed
refusal names the entity and the reason, so the gap is visible.

If you hit one of these, the fix is in the authoring tool or the file, not
here.

## Cases

### A parameter-space conic positioned by a 3D placement

`IfcPcurve.ReferenceCurve` is drawn in the basis surface's own two-dimensional
`(u, v)` parameter domain, not in model space. When that reference curve is an
`IfcCircle` or `IfcEllipse`, it inherits `Position` from `IfcConic`:

```
IfcConic.Position : IfcAxis2Placement    -- = IfcAxis2Placement2D | IfcAxis2Placement3D
```

That is a SELECT admitting either dimensionality, and no `WHERE` rule narrows
it when the conic is used as a p-curve reference. So a file may legally supply
an `IfcAxis2Placement3D` -- carrying an origin `(x, y, z)` and an `Axis`
direction -- to position a curve inside a domain that has no third dimension.

The lenient reading is to drop `z` and keep `(x, y)`. That is wrong, not
merely lossy: `Axis` *orients* the placement, so discarding it does not
project the frame onto the domain, it just abandons the orientation and keeps
whatever numbers are left. The result is a plausible conic at the wrong angle.
Handling it correctly would require inventing a projection from a 3D frame
onto `(u, v)`, which IFC does not define, because the combination is not
meaningful in the first place.

The schema is not silent about p-curve dimensionality in general. It
declares `DimIs2D : ReferenceCurve.Dim = 2` on `IfcPcurve`, which the
executable inventory in `ifc-geometry/data/ifc4-where-rules.tsv` records.
The gap is narrower than it first appears: the reference curve must be 2D,
but nothing constrains the dimensionality of the *placement* that positions
a conic reference curve. A 3D placement can therefore sit under a curve
that is itself correctly 2D.

### A surface curve whose master names a side it does not have

`IfcSurfaceCurve.MasterRepresentation` may be `PCURVE_S2` while
`AssociatedGeometry` holds only one p-curve. The master then names a
parametric side the curve does not carry.

IFC itself calls this inconsistent, but the constraint is not expressible in
the attribute declaration, so a file can assert it. Resolving `PCURVE_S2` to
the single p-curve present would silently reinterpret the authored master as
`PCURVE_S1`, changing which surface the curve is understood to lie on.

## Related, but not schema faults

Two neighbouring refusals look similar and are not listed above, because the
schema is not at fault:

- **A bare supertype instance.** `IfcProfileDef`, `IfcLoop` and `IfcVertex`
  are instantiable but declare no concrete geometry. A file authoring one has
  supplied a label, not a shape. The schema is behaving as intended; there is
  simply nothing to lower.
- **Convention-only `IfcBSplineCurve`.** This entity is `ABSTRACT` and cannot
  be instantiated at all. What is refused in parameter space is the
  *convention-only* reading of its concrete subtypes, which carries no
  authored knot vector to preserve -- an unimplemented dimensional contract,
  not a schema defect.

Everything else this library declines is either unimplemented or awaiting an
upstream kernel contract. See [Capabilities](/capabilities) for the full
per-entity table with the exact rationale attached to each variant.
