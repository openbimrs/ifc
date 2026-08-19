# Plan — `ifc-geometry`: the IFC side of geometry

## Goal

Implement the IFC interpretation of the three geometry resource schemas so that
a geometry kernel (built separately) knows **exactly what it must support**.

`ifc-geometry` answers "what does this IFC entity *mean* geometrically" and
emits kernel-neutral work orders. It never rasterises, never triangulates,
never does a boolean. That is the kernel's job.

## Scope (counted from IFC4 ADD2 TC1, not guessed)

| Schema | Entities | Types | Functions |
| --- | ---: | ---: | ---: |
| `IfcGeometryResource` | 59 | 14 | 25 |
| `IfcGeometricModelResource` | 42 | 4 | 2 |
| `IfcGeometricConstraintResource` | 11 | 5 | 1 |
| **Total** | **112** | **23** | **28** |

23 abstract supertypes, 89 concrete. Source of truth:
`references/ifc-spec/ifc4-add2-tc1/IFC4.exp` plus the HTML docs under
`dist/ifc4-add2-tc1/html/schema/<resource>/lexical/`.

## Architecture

```
        ifc-model (untyped entity graph)
              |
              v
   +---------------------------+
   |  resource/  typed views   |   borrow &Entity, zero copy, no owned data
   |  constraint/              |
   +---------------------------+
              |
              v
   +---------------------------+
   |  units/      scale resolution
   |  placement/  chain composition + cycle detection + cache
   +---------------------------+
              |
              v
   +---------------------------+
   |  lower/   -> KernelRequest |  kernel-neutral work orders
   +---------------------------+
              |
              v
        geom-kernel TRAIT (implemented elsewhere)
```

### Key decisions

1. **Typed views, not owned structs.** Each entity gets a newtype wrapping
   `(EntityId, &Entity)` with named accessors. Same pattern as `ifc-cost`.
   Attribute indices are `mod slot` constants citing the EXPRESS line.

2. **The kernel is a trait we *demand*, not one we implement.** `kernel/`
   defines the capability surface the geometry package must satisfy. This file
   is the deliverable for the other session.

3. **Lowering is total.** Every unsupported case returns a typed error naming
   the entity; nothing panics, nothing silently produces wrong geometry.

4. **Resolution is separate from representation.** `resource/` only reads the
   file. `placement/` and `units/` interpret. Keeps parsing testable without a
   kernel.

## Semantics that are easy to get silently wrong

Captured from the spec prose; each gets an explicit test.

- **`IfcHalfSpaceSolid` is unbounded.** Only usable inside a boolean; the
  kernel needs a clipping volume. `IfcPolygonalBoundedHalfSpace` bounds it by
  a 2D polygon in the position XY plane extruded along +Z.
- **`IfcTrimmedCurve.SenseAgreement`** changes the *shape* on closed basis
  curves (circle/ellipse), not just direction. Four different arcs from the
  same basis curve + trim points.
- **`IfcLocalPlacement.PlacementRelTo` absent ⇒ world coordinates.** Cycles are
  possible in real files; the spec explicitly pushes cycle prevention to the
  application. Must detect, not stack-overflow.
- **`IfcMappedItem` nests.** A representation map may itself contain mapped
  items. Depth-limit and cycle-detect.
- **Units scale coordinates.** `IfcSIUnit` prefix (`MILLI`, `KILO`) and
  `IfcConversionBasedUnit` change the meaning of every raw number.
- **`IfcExtrudedAreaSolid.ExtrudedDirection`** is in the *position* coordinate
  system and may be oblique — not necessarily +Z.
- **Trim parameters** may be Cartesian points OR parameter values; preference
  given by `IfcTrimmingPreference`.

## Module layout

`lib.rs` stays a facade (<40 code lines, enforced by the gate). MAX_LINES=800
per file, so families split into directories.

```
resource/{point,direction,placement,transform}.rs
resource/curve/{conic,line,bounded,offset,surface_curve}.rs
resource/surface/{elementary,swept,bounded}.rs
resource/solid/{swept_area,swept_disk,brep,csg,halfspace,primitive}.rs
resource/{tessellated,surface_model,mapped,bbox}.rs
constraint/{object_placement,grid,connection}.rs
units/, placement/, lower/, kernel/
```

## Stages

| # | Stage | State | Verification |
| --- | --- | --- | --- |
| 1 | Foundation: `error`, `slots`, `units` | **done** | 20 tests, all 19 fixtures |
| 2 | `transform` + `kernel` request vocabulary | **done** | 25 tests |
| 3 | `constraint/`: local placement, grid, connection | **in progress** | cycle + composition tests |
| 4 | `resource/`: points, directions, placements, operators | delegated | |
| 5 | `curve/` + `surface/` | delegated | |
| 6 | `solid/`: swept, brep, csg, halfspace, boolean, tessellated | delegated | |
| 7 | `lower/`: views to `kernel::Primitive` | pending | request per fixture entity |
| 8 | Inventory test: all 112 entities recognised | pending | the honesty check |

Stage 8 is the honesty check: a test enumerating every concrete entity in the
three schemas and asserting the dispatcher recognises it, so "we support the
geometry resources" becomes a claim the build verifies rather than a claim I
make.

## Reference generated for implementers

`references/absolute-slots.txt` lists the **absolute** positional STEP index of
every attribute of every concrete geometry entity, inherited attributes first,
generated from `IFC4.exp`. This exists because the EXPRESS declaration of, say,
`IfcExtrudedAreaSolid` lists only `ExtrudedDirection` and `Depth`, while the
STEP record is `(SweptArea, Position, ExtrudedDirection, Depth)` -- the two
inherited slots come first. Reading local indices off the schema and using them
as record positions misreads every solid in the file, silently.

Verified against a real record:
`#338081= IFCEXTRUDEDAREASOLID(#338077,#338080,#19,2.41)`.
