# 2D approval plans (Baugenehmigung)

**Scenario.** A user imports an IFC model, derives 2D floor plans from it,
annotates those plans with the construction symbols required for a German
building-permit submission (*Baugenehmigung*), and keeps the result IFC-native
— symbols carried as `IfcAnnotation`, symbol definitions referenced through
`IfcLibraryReference`, and sign-off recorded with `IfcApproval`.

This page maps that scenario onto the current code. It is written to be read
before the first line of application code is written.

## Verdict up front

`openbim-ifc` gives you a **correct, lossless IFC substrate**, **real 2D curve
geometry**, **schema-checked authoring**, **spatial traversal**, and bounded
typed **presentation, annotation, and library semantics**. It does **not** give
you plan sectioning, annotation placement, rendering, or approval semantics.

That distinction matters for this use case. You can now construct any entity the
schema declares — including `IfcAnnotation`, `IfcCurveStyle`,
`IfcLibraryReference` and `IfcApproval` — by naming its attributes, and the
builder will refuse a wrong arity or type. `ifc-style` adds strict borrowed views
and selected transaction-staged writers for annotations and presentation data;
`ifc-classification` does the same for library references. The repository still
lacks the sectioning geometry needed to derive a plan from a 3D body.

| Scenario step | Served by this crate? |
| --- | --- |
| Read `.ifc` / `.ifcxml` losslessly | <span class="status-implemented">Yes</span> |
| Preserve every entity you do not interpret | <span class="status-implemented">Yes</span> |
| Find products and their representations | <span class="status-partial">Partly</span> |
| Walk the spatial tree (storey → elements) | <span class="status-implemented">Yes</span> — via `ifc-spatial` |
| Read 2D curve *attributes* (`IfcPolyline`, `IfcCircle`, …) | <span class="status-implemented">Yes</span> |
| Lower supported nested curves into the neutral geometry graph | <span class="status-partial">Partly</span> — bare curves are not top-level body targets |
| Select the 2D (`Plan`/`Annotation`) representation | <span class="status-implemented">Yes</span> — via `select_plan_representation` |
| Cut a section / derive a plan from 3D bodies | <span class="status-absent">No</span> |
| Author `IfcAnnotation` symbols | <span class="status-implemented">Yes</span> — bounded helpers in `ifc-style`; generic fallback in `ifc-author` |
| *Write* style entities (`IfcCurveStyle`, …) | <span class="status-implemented">Yes</span> — selected bounded helpers in `ifc-style`; generic fallback in `ifc-author` |
| *Interpret* style entities as a typed view | <span class="status-implemented">Yes</span> — bounded presentation domain in `ifc-style` |
| *Write* `IfcLibraryReference` | <span class="status-implemented">Yes</span> — via `ifc-classification` or generic `ifc-author` |
| *Write* `IfcApproval` | <span class="status-implemented">Yes</span> — generic `ifc-author` only |
| *Interpret* library / approval entities as typed views | <span class="status-partial">Partly</span> — library views exist; approval views do not |
| Write the result back out | <span class="status-implemented">Yes</span> |

## What works today

### Lossless import and export

This is the strongest guarantee the project makes, and it is the one that makes
the rest of the gaps survivable. `ifc-model` stores entities structurally — a
type name plus positional attribute values — so entities the application never
interprets are still preserved byte-faithfully through a read/write cycle.

Practically: your application can read a model containing `IfcAnnotation`,
`IfcApproval`, and `IfcLibraryReference` entities authored by *another* tool,
modify only what it understands, and write the file back **without destroying
them**. That property holds today, with no domain crate compiled.

```rust
use ifc::{Codec, StepCodec};

let model = StepCodec.read_bytes(source)?;

// Entities of any type survive, interpreted or not.
let annotations = model.ids_of_type("IFCANNOTATION");
println!("{} annotations passed through untouched", annotations.len());

let out = StepCodec.write_bytes(&model)?;
# Ok::<(), ifc::ModelError>(())
```

Note `ids_of_type` takes the **upper-case** STEP type name.

### 2D curve geometry

`ifc-geometry` provides borrowed typed views for the curve families used by 2D
plans and exact neutral-graph lowerers for a bounded subset. `lower/curve.rs`
lowers `IfcPolyline`, `IfcLine`, `IfcCircle`, `IfcTrimmedCurve`,
`IfcCompositeCurve`, `IfcBSplineCurveWithKnots`, and
`IfcRationalBSplineCurveWithKnots`. The explicit-knot B-spline paths preserve
degree, compact knots and multiplicities, control points, optional rational
weights, and representable closure.

Curves reach that lowerer through the item that owns them: for example a sweep
directrix, surface boundary, B-rep edge, or `IfcGeometricCurveSet` member. A
bare curve is deliberately not a top-level *body* dispatch target, so offering
a standalone `IfcPolyline` as a body still returns typed `Unsupported` rather
than pretending a line is a solid.

This is neutral geometry, not a complete permit-drawing pipeline. The crate does
not section 3D bodies, interpret curve styles, place annotations, or render the
result; an application must still provide those layers.

| Entity | Typed view | Neutral lowering |
| --- | --- | --- |
| `IfcPolyline` | Yes | Yes, through an owning item |
| `IfcCircle` | Yes | Yes, through an owning item |
| `IfcEllipse` | Yes | Not yet |
| `IfcLine` | Yes | Yes, through an owning item |
| `IfcTrimmedCurve` | Yes | Yes, through an owning item |
| `IfcCompositeCurve` | Yes | Yes, through an owning item |
| Convention-only `IfcBSplineCurve` | Yes | No — absent knots are not invented |
| `IfcBSplineCurveWithKnots`, `IfcRationalBSplineCurveWithKnots` | Yes | Yes, exact neutral values |
| `IfcOffsetCurve2D` | Yes | Not yet |
| `IfcAxis2Placement2D` | Yes | Placement support |
| `IfcCartesianTransformationOperator2D` | Yes | Transform support |

Unit resolution (`units.rs`) handles SI and conversion-based units, which
matters because plan dimensions in millimetres versus metres is a classic
silent-corruption bug in permit drawings.

### Reading a product's representations

`ifc-geometry::input::representation` gives you `ProductShape` and
`Representation` views, so you can enumerate a product's representations and
read each one's `RepresentationIdentifier` (`Body`, `Axis`, `FootPrint`, …).

## The four things you must build

### 1. ~~A 2D representation selector~~ — provided

`select_shape_representation` is written for a **3D viewer** and deliberately
refuses `Axis`/`FootPrint`. `select_plan_representation` is its inverse and now
ships:

```rust
use ifc::{select_plan_representation, plan_contexts};   // feature = "geometry-select"

let drawable = select_plan_representation(&model, wall)?;
```

It prefers a drawable identifier authored into an explicit `PLAN_VIEW`
sub-context, then falls back to `Plan`, `Annotation`, `FootPrint`, `Axis` in
that order. The two conditions are intersected: exporters write
`Box`/`BoundingBox` representations into plan contexts, and a bounding box is
not a drawing whatever context it sits in.

Selection is the light half of the crate. `geometry-select` gives you
contexts, plan/body choice, placements and units without compiling a solid
modeller; `geometry` adds lowering into the neutral DAG:

```toml
ifc = { version = "0.1", features = ["step", "geometry-select"] }
```

Measured on this repository's own drawing app, that is 107 crates instead of
116, and zero `axiolid-*` kernel crates instead of nine.

`None` means the product carries only solid or bounding-box geometry. That is a real answer:
turning a solid into a plan needs sectioning, which is still §3 below.

### Where a product sits in the world

Selecting a representation gives geometry in the product's own local space. To
draw it you need the world transform, which means resolving the
`IfcLocalPlacement` chain up through storey, building and site:

```rust
use ifc::{product_world_transform, products_world_transforms};   // feature = "geometry-select"

let world = product_world_transform(&model, &units, wall)?;
```

Do not hand-roll this. The two mistakes are invisible until late: composing the
chain innermost-first mirrors the model about its ancestors, and applying the
unit scale per link instead of once raises the factor to the power of the chain
depth, so a millimetre file three levels deep lands a thousand times too far
out. Cyclic chains in malformed files are reported rather than hung on.

For a whole-model walk use the batch form, which shares one placement cache --
every product in a storey shares that storey's entire ancestor chain:

```rust
for (id, world) in products_world_transforms(&model, &units, ids) {
    // Errors are per-product: one broken chain does not suppress the rest.
}
```

Plan contexts are readable too, including the `*` inheritance real exporters
write:

```rust
for context in plan_contexts(&model) {
    context.target_scale();       // Some(0.01) for 1:100
    context.precision(&model);    // resolved from the parent context
}
```

### 2. ~~Spatial tree traversal~~ — provided

`ifc-spatial` builds the tree. Grouping elements by storey, the query a floor
plan is organised around, is:

```rust
use ifc::{SpatialKind, SpatialTree};   // feature = "spatial"

let tree = SpatialTree::build(&model);
for storey in tree.of_kind(SpatialKind::Storey) {
    let on_this_level = tree.elements_of(storey.id);
}
```

`container_of(element)` answers the inverse — which storey a given wall is on —
and `elements_recursive` descends through spaces. Anomalies a permit drawing
would care about are reported rather than hidden: `orphans()` lists containers
nothing aggregates, `dangling()` lists relationships naming absent entities.

One caution specific to this use case: a real corpus file was found that
declares its whole hierarchy with `IfcRelAggregates` and contains **no**
containment relationship at all. `elements_of` is correctly empty there. If your
input comes from such an exporter, elements are associated by other means and
you must handle that case explicitly rather than assuming an empty storey is a
bug in the library.

### 3. Plan derivation from 3D geometry

If the source model has no `FootPrint`/`Annotation` representation — common —
you must derive plan geometry by sectioning the 3D bodies at a cut height.

Nothing in this repository does that. `ifc-geometry` lowers bodies into the
neutral [Axiolid](/architecture/axiolid-boundary) DAG and stops; sectioning is a
kernel operation. Two further constraints:

- Representation-item lowering remains partial overall, but
  `IfcTriangulatedFaceSet` and `IfcPolygonalFaceSet` — heavily used by IFC4
  exporters — both lower today. Consult the generated
  [dispatch table](/capabilities#representation-item-lowering) for the exact
  supported families.
- A plane/solid section operation is an Axiolid capability question, not an IFC
  one. Check Axiolid's own capability page before assuming it exists.

::: tip Reduce scope where you can
If your product can require that incoming models carry authored 2D
representations, steps 1 and 3 shrink dramatically. Many architectural exports
do include `FootPrint`. Consider making full 3D sectioning a later milestone
rather than a launch requirement.
:::

### 4. Authoring: annotations, styles, libraries, approvals

**This gap is closed for writing.** `ifc-style` provides bounded transactional
helpers for the named annotation and core style graphs. For entities outside
that surface, `ifc-author` constructs any schema-declared entity by name:

```rust
use ifc::EntityBuilder;               // feature = "author"

let annotation = EntityBuilder::new(&schema, "IfcAnnotation")
    .text("GlobalId", "3vB2YO$MX4xv5uCqZZG05x")
    .text("Name", "Brandwand")
    .insert(&mut model)?;
```

Seven slots are produced because IFC4 declares seven, with `GlobalId` first
because it is inherited from `IfcRoot`. A typo in the entity or attribute name,
a duplicate set, a missing required attribute, a wrong value type, or a
malformed GlobalId are all refused before anything reaches the model.

That covers every entity in this list — they are all declared by the schema:

| What you must write | Can you author it? | Is there a typed view to read it back? |
| --- | --- | --- |
| `IfcAnnotation` | <span class="status-implemented">Yes</span> | <span class="status-implemented">Yes</span> (`ifc-style`) |
| `IfcTextLiteralWithExtent` | <span class="status-implemented">Yes</span> | <span class="status-implemented">Yes</span> (`ifc-style`) |
| `IfcAnnotationFillArea` | <span class="status-implemented">Yes</span> | <span class="status-implemented">Yes</span> (`ifc-style`) |
| `IfcCurveStyle`, `IfcFillAreaStyle`, `IfcStyledItem` | <span class="status-implemented">Yes</span> | <span class="status-implemented">Yes</span> (`ifc-style`) |
| `IfcPresentationLayerAssignment` | <span class="status-implemented">Yes</span> | <span class="status-implemented">Yes</span> (`ifc-style`) |
| `IfcShapeRepresentation` | <span class="status-implemented">Yes</span> | <span class="status-absent">No</span> |
| `IfcGeometricRepresentationSubContext` | <span class="status-implemented">Yes</span> | <span class="status-implemented">Yes</span> (`ifc-geometry`) |
| `IfcLibraryReference`, `IfcLibraryInformation` | <span class="status-implemented">Yes</span> | <span class="status-implemented">Yes</span> (`ifc-classification`) |
| `IfcRelAssociatesLibrary` | <span class="status-implemented">Yes</span> | <span class="status-implemented">Yes</span> (`ifc-classification`) |
| `IfcApproval`, `IfcRelAssociatesApproval` | <span class="status-implemented">Yes</span> | <span class="status-absent">No</span> |

Rows with a typed view expose domain meaning and strict reference/value errors.
The remaining absent rows still round-trip structurally through `ifc-model`, but
your application owns their interpretation.

::: tip What to still verify yourself
`ifc-author` checks arity and declared types. It does not check WHERE rules,
inverse attributes, or whether your `IfcRelAssociatesApproval` actually points
at a sensible object. Round-trip your output through `StepCodec` and open it in
a reference tool before trusting it as a permit document.
:::

## Before you export: will a viewer actually draw it?

A file can be schema-valid, pass `IfcOpenShell.validate` with zero errors, and
still open completely blank. Validation asks whether the file is legal IFC.
Whether the geometry is *reachable* is a different question, and nothing in the
schema answers it.

```rust
use ifc::{unreachable_products, Codec, StepCodec};

let model = StepCodec.read_bytes(&bytes)?;
for (id, why) in unreachable_products(&model) {
    eprintln!("#{id}: {}", why.message());
}
```

Enable with `features = ["step", "spatial", "geometry-select"]`.

Three causes are reported:

| Cause | Why it is invisible |
| --- | --- |
| `NotContainedInSpatialStructure` | Viewers reach geometry by walking the spatial tree. A product outside it is never visited, however well-formed. |
| `NoRepresentationInModelContext` | A body authored only into `PlanView` is skipped by a 3D viewer, which renders `Model`. |
| `RepresentationWithoutContext` | Geometry a viewer cannot schedule, because the context reference does not resolve. |

The second one is the trap specific to this use case. Annotations *belong* in a
plan sub-context — that is correct authoring — but if the product's **body**
also lives only there, the model viewer you check your work in shows nothing,
and it is easy to conclude the export failed entirely.

::: tip What it will not tell you
Being outside the spatial tree is normal for openings (`IfcRelVoidsElement`),
assembly parts (`IfcRelAggregates`), spatial containers themselves, and
products with no representation. None of these are reported: on
`AC20-FZK-Haus.ifc`, 20 of 127 products sit outside the containment tree and
the lint finds nothing, because all 20 are legitimate. A lint that cries wolf
is one you switch off.
:::

## Recommended build order

1. **Import + export round-trip.** Prove a file survives your pipeline
   unchanged before adding behaviour. Use `openbim-ifc` with
   `features = ["step", "ifcxml", "author"]`.
2. ~~**Thin authoring layer.**~~ No longer yours to build: enable
   `features = ["step", "author"]` and use `EntityBuilder`.
3. **Spatial grouping.** Storey → elements, written against
   `IFCRELCONTAINEDINSPATIALSTRUCTURE`.
4. ~~**2D representation selection.**~~ Provided: `select_plan_representation`.
5. **Annotation authoring.** `IfcAnnotation` + `IfcTextLiteralWithExtent` +
   `IfcCurveStyle`, assigned into an annotation sub-context.
6. **Library references.** `IfcLibraryReference` per symbol, associated via
   `IfcRelAssociatesLibrary`. This is what makes your symbol set portable rather
   than proprietary.
7. **Approvals.** `IfcApproval` + `IfcRelAssociatesApproval` for sign-off state.
8. *(Later)* **Plan derivation from 3D.** Tessellated face-set lowering is
   available; a sectioning path is still required.

## What would change this verdict

The repository now owns the lossless substrate, plan-representation selection,
bounded annotation/presentation semantics, and library-reference domain. The
remaining project-level gaps for this scenario are 3D-to-plan sectioning and a
typed approval domain; placement, composition, and rendering remain
application/provider responsibilities.

If you are building this application, those remaining [roadmap](/project/roadmap)
items are the ones worth watching — or contributing to. See
[contributing](/guide/contributing).
