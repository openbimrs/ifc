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
geometry**, and **schema-checked authoring**. It does **not** give you the
presentation, annotation, library, or approval *semantics*.

That distinction matters for this use case. You can now construct any entity the
schema declares — including `IfcAnnotation`, `IfcCurveStyle`,
`IfcLibraryReference` and `IfcApproval` — by naming its attributes, and the
builder will refuse a wrong arity or type. What no crate here provides is a
typed *view* that interprets those entities once written, or the geometry to
derive a plan in the first place.

| Scenario step | Served by this crate? |
| --- | --- |
| Read `.ifc` / `.ifcxml` losslessly | <span class="status-implemented">Yes</span> |
| Preserve every entity you do not interpret | <span class="status-implemented">Yes</span> |
| Find products and their representations | <span class="status-partial">Partly</span> |
| Walk the spatial tree (storey → elements) | <span class="status-absent">No</span> |
| Read 2D curve *attributes* (`IfcPolyline`, `IfcCircle`, …) | <span class="status-implemented">Yes</span> |
| Lower a curve into the geometry graph | <span class="status-absent">No</span> |
| Select the 2D (`Plan`/`Annotation`) representation | <span class="status-absent">No</span> |
| Cut a section / derive a plan from 3D bodies | <span class="status-absent">No</span> |
| Author `IfcAnnotation` symbols | <span class="status-implemented">Yes</span> — via `ifc-author` |
| *Write* style entities (`IfcCurveStyle`, …) | <span class="status-implemented">Yes</span> — via `ifc-author` |
| *Interpret* style entities as a typed view | <span class="status-scaffold">Scaffold only</span> |
| *Write* `IfcLibraryReference` / `IfcApproval` | <span class="status-implemented">Yes</span> — via `ifc-author` |
| *Interpret* them as typed views | <span class="status-absent">No</span> |
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

`ifc-geometry` implements the curve families a 2D plan needs, as borrowed views
over the model that resolve units and placements. **These are readers, not
lowerers**: `lower/curve.rs` is a placeholder and the dispatcher handles no
curve family, so lowering a top-level `IfcPolyline` returns `Unsupported`. The
one exception is a polyline used as a swept-solid profile outline
(`lower/profile.rs`).

You can therefore read every coordinate you need and build your own 2D
pipeline on top — you just cannot ask this crate to produce drawable geometry
for you.

| Entity | Module |
| --- | --- |
| `IfcPolyline` | `curve/polyline.rs` |
| `IfcCircle`, `IfcEllipse` | `curve/conic.rs` |
| `IfcLine` | `curve/line.rs` |
| `IfcTrimmedCurve` | `curve/trimmed.rs` |
| `IfcCompositeCurve` | `curve/composite.rs` |
| `IfcOffsetCurve2D` | `curve/offset.rs` |
| `IfcAxis2Placement2D` | `resource/placement.rs` |
| `IfcCartesianTransformationOperator2D` | `resource/operator.rs` |

Unit resolution (`units.rs`) handles SI and conversion-based units, which
matters because plan dimensions in millimetres versus metres is a classic
silent-corruption bug in permit drawings.

### Reading a product's representations

`ifc-geometry::input::representation` gives you `ProductShape` and
`Representation` views, so you can enumerate a product's representations and
read each one's `RepresentationIdentifier` (`Body`, `Axis`, `FootPrint`, …).

## The four things you must build

### 1. A 2D representation selector

The crate ships `select_shape_representation`, which is written for a **3D
viewer** and does the opposite of what you need:

```rust
// From ifc-geometry/src/input/representation.rs
pub const SOLID_IDENTIFIERS: &[&str] = &["Body", "Facetation"];

// Axis and FootPrint are deliberately absent: they are 2D
// annotations, and selecting one silently replaces a solid with a line.
```

That policy is correct for its stated purpose and wrong for yours. You need the
inverse: prefer `FootPrint`, `Axis`, `Annotation`, `Plan`. There is no supported
API for this, but the building blocks (`ProductShape`, `Representation`,
`identifier()`) are public, so the selector is perhaps 40 lines you own:

```rust
use ifc_geometry::input::representation::{ProductShape, Representation};

/// Representation identifiers that carry 2D plan geometry, best first.
const PLAN_IDENTIFIERS: &[&str] = &["FootPrint", "Annotation", "Plan", "Axis"];
```

Walk `ProductShape::representations()`, wrap each in `Representation::new`, and
match `identifier()` against that list in order.

### 2. Spatial tree traversal

To produce a plan *per storey* you must group elements by `IfcBuildingStorey`.
That means following `IfcRelContainedInSpatialStructure` and `IfcRelAggregates`.

`ifc-model/src/spatial.rs` and `relation.rs` are placeholders — the latter says
"Not yet implemented" in its own doc comment. There is also no reverse-reference
index, so "which relationship entities point at this storey" is currently a
linear scan you write yourself.

Concretely, you must:

- iterate `model.ids_of_type("IFCRELCONTAINEDINSPATIALSTRUCTURE")`;
- read attribute 4 (`RelatedElements`, a list of refs) and attribute 5
  (`RelatingStructure`, a ref);
- build your own storey → elements map.

The same pattern applies to `IFCRELAGGREGATES` for project → site → building →
storey. Budget this as real work, and be aware real files omit levels or attach
elements directly to the building.

### 3. Plan derivation from 3D geometry

If the source model has no `FootPrint`/`Annotation` representation — common —
you must derive plan geometry by sectioning the 3D bodies at a cut height.

Nothing in this repository does that. `ifc-geometry` lowers bodies into the
neutral [Axiolid](/architecture/axiolid-boundary) DAG and stops; sectioning is a
kernel operation. Two further constraints:

- Only seven representation-item families lower today. Notably
  `IfcTriangulatedFaceSet` and `IfcPolygonalFaceSet` — heavily used by IFC4
  exporters — return `GeometryError::Unsupported`. See the
  [dispatch table](/capabilities#representation-item-lowering).
- A plane/solid section operation is an Axiolid capability question, not an IFC
  one. Check Axiolid's own capability page before assuming it exists.

::: tip Reduce scope where you can
If your product can require that incoming models carry authored 2D
representations, steps 1 and 3 shrink dramatically. Many architectural exports
do include `FootPrint`. Consider making full 3D sectioning a later milestone
rather than a launch requirement.
:::

### 4. Authoring: annotations, styles, libraries, approvals

**This gap is closed for writing.** `ifc-author` constructs any entity the
schema declares, by name:

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
| `IfcAnnotation` | <span class="status-implemented">Yes</span> | <span class="status-absent">No</span> |
| `IfcTextLiteralWithExtent` | <span class="status-implemented">Yes</span> | <span class="status-absent">No</span> |
| `IfcAnnotationFillArea` | <span class="status-implemented">Yes</span> | <span class="status-absent">No</span> |
| `IfcCurveStyle`, `IfcFillAreaStyle`, `IfcStyledItem` | <span class="status-implemented">Yes</span> | <span class="status-scaffold">Scaffold</span> (`ifc-style`) |
| `IfcPresentationLayerAssignment` | <span class="status-implemented">Yes</span> | <span class="status-scaffold">Scaffold</span> |
| `IfcShapeRepresentation` | <span class="status-implemented">Yes</span> | <span class="status-absent">No</span> |
| `IfcGeometricRepresentationSubContext` | <span class="status-implemented">Yes</span> | <span class="status-absent">No</span> |
| `IfcLibraryReference`, `IfcLibraryInformation` | <span class="status-implemented">Yes</span> | <span class="status-absent">No</span> |
| `IfcRelAssociatesLibrary` | <span class="status-implemented">Yes</span> | <span class="status-absent">No</span> |
| `IfcApproval`, `IfcRelAssociatesApproval` | <span class="status-implemented">Yes</span> | <span class="status-absent">No</span> |

The remaining column is the honest one: **nothing interprets these entities once
written.** Your application holds the meaning. For generating permit documents
that is usually acceptable — you wrote them, so you know what they are — but a
round-trip through another tool and back will hand you entities this library
preserves losslessly and does not explain.

::: tip What to still verify yourself
`ifc-author` checks arity and declared types. It does not check WHERE rules,
inverse attributes, or whether your `IfcRelAssociatesApproval` actually points
at a sensible object. Round-trip your output through `StepCodec` and open it in
a reference tool before trusting it as a permit document.
:::

## Recommended build order

1. **Import + export round-trip.** Prove a file survives your pipeline
   unchanged before adding behaviour. Use `openbim-ifc` with
   `features = ["step", "ifcxml", "author"]`.
2. ~~**Thin authoring layer.**~~ No longer yours to build: enable
   `features = ["step", "author"]` and use `EntityBuilder`.
3. **Spatial grouping.** Storey → elements, written against
   `IFCRELCONTAINEDINSPATIALSTRUCTURE`.
4. **2D representation selection.** The inverse selector.
5. **Annotation authoring.** `IfcAnnotation` + `IfcTextLiteralWithExtent` +
   `IfcCurveStyle`, assigned into an annotation sub-context.
6. **Library references.** `IfcLibraryReference` per symbol, associated via
   `IfcRelAssociatesLibrary`. This is what makes your symbol set portable rather
   than proprietary.
7. **Approvals.** `IfcApproval` + `IfcRelAssociatesApproval` for sign-off state.
8. *(Later)* **Plan derivation from 3D**, once the tessellated face-set families
   lower and a sectioning path exists.

## What would change this verdict

Items 1, 4, 5, 6, and 7 above are all things this repository *should* own rather
than each application reinventing. They are tracked on the
[roadmap](/project/roadmap) as the authoring layer, the presentation/annotation
domain, and the external-reference domain.

If you are building this application, those roadmap items are the ones worth
watching — or contributing to. See [contributing](/guide/contributing).
