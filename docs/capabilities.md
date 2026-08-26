# Capabilities and status

This page is deliberately conservative. It exists so that an engineer — or a
coding agent — can scope work against what the code *does*, not against what the
IFC schema contains or a crate name suggests.

## Status vocabulary

| Term | Meaning |
| --- | --- |
| **Implemented** | Executable behaviour with tests. Safe to build on. |
| **Partial** | A real vertical slice exists; named gaps return typed errors. |
| **Scaffold** | Module and crate ownership exist. **No behaviour.** Files are doc-comment placeholders that reserve a name so the architecture is reviewable. |
| **Absent** | Not present in the workspace at all. Not even a reserved name. |

A scaffold crate compiles, publishes, and appears in the feature list. It does
**not** read, write, or interpret the entities its module names refer to.

## Workspace census

Measured from the source tree, not estimated. "Stub files" counts source files
of twelve lines or fewer — the placeholder shape described above.

| Crate | Source LOC | Files | Stub files | Test files | Status |
| --- | ---: | ---: | ---: | ---: | --- |
| `ifc-geometry` | 19,162 | 77 | 9 | 15 | <span class="status-partial">Partial</span> |
| `ifc-template-catalog` | 2,403 | 28 | 3 | 9 | <span class="status-implemented">Implemented</span> |
| `ifc-material` | 2,114 | 23 | 0 | 7 | <span class="status-implemented">Implemented</span> |
| `ifc-model` | 841 | 24 | 14 | 5 | <span class="status-implemented">Implemented</span> |
| `ifc-xml` | 658 | 4 | 0 | 1 | <span class="status-implemented">Implemented</span> |
| `ifc-cost` | 482 | 8 | 0 | 0 | <span class="status-partial">Partial</span> |
| `ifc-schema` | 373 | 8 | 4 | 1 | <span class="status-implemented">Implemented</span> |
| `ifc-step` | 341 | 4 | 0 | 1 | <span class="status-implemented">Implemented</span> |
| `openbim-ifc` | 304 | 4 | 0 | 6 | <span class="status-implemented">Implemented</span> |
| `ifc-properties` | 210 | 29 | 24 | 0 | <span class="status-scaffold">Scaffold</span> |
| `ifc-structural` | 194 | 27 | 22 | 0 | <span class="status-scaffold">Scaffold</span> |
| `ifc-alignment` | 186 | 26 | 21 | 0 | <span class="status-scaffold">Scaffold</span> |
| `ifc-resource` | 177 | 25 | 23 | 0 | <span class="status-scaffold">Scaffold</span> |
| `ifc-style` | 174 | 24 | 20 | 0 | <span class="status-scaffold">Scaffold</span> |
| `ifc-validate` | 172 | 23 | 18 | 0 | <span class="status-scaffold">Scaffold</span> |
| `ifc-schedule` | 170 | 23 | 21 | 0 | <span class="status-scaffold">Scaffold</span> |
| `ifc-systems` | 149 | 20 | 18 | 0 | <span class="status-scaffold">Scaffold</span> |
| `ifc-georef` | 133 | 17 | 14 | 0 | <span class="status-scaffold">Scaffold</span> |
| `ifc-classification` | 131 | 17 | 14 | 0 | <span class="status-scaffold">Scaffold</span> |

Nine of nineteen crates are scaffolds. They exist because the layering decision
(see [ADR 0005](/adr/0005-scaffold-modules-declare-ownership))
prefers declaring the intended home of a domain up front over discovering it
later, but they must never be mistaken for working code.

## Core: model, codecs, schema

| Capability | Status | Evidence |
| --- | --- | --- |
| Entity graph with positional attributes | <span class="status-implemented">Implemented</span> | `ifc-model::Model` |
| Round-trip of entities the build does not understand | <span class="status-implemented">Implemented</span> | `openbim-ifc/tests/costing_roundtrip.rs` (runs with no domain crate compiled) |
| STEP (`.ifc`) read and write | <span class="status-implemented">Implemented</span> | `ifc-step`, delegating ISO 10303-21 syntax to `openbim-step` |
| ifcXML read and write | <span class="status-implemented">Implemented</span> | `ifc-xml` |
| IFC-JSON | <span class="status-absent">Absent</span> | Would be a third `Codec` impl; no crate exists |
| EXPRESS schema metadata, subtype queries | <span class="status-implemented">Implemented</span> | `ifc-schema` |
| GlobalId base-64 encode/decode | <span class="status-implemented">Implemented</span> | `ifc-model::guid` |
| Spatial containment tree traversal | <span class="status-implemented">Implemented</span> | `ifc-spatial::SpatialTree`; facade feature `spatial`. See below. |
| Objectified relationship traversal | <span class="status-partial">Partial</span> | `ifc-spatial::relation` reads `IfcRelAggregates`, `IfcRelContainedInSpatialStructure`, `IfcRelNests`. Other `IfcRel*` families are not interpreted. |
| Cycle-protected graph walks | <span class="status-scaffold">Scaffold</span> | `ifc-model/src/traverse.rs` |
| Type index (`ids_of_type`, `of_type`) | <span class="status-implemented">Implemented</span> | `Model::ids_of_type`, backed by `index/type_index.rs` |
| Reverse-reference index ("who references me") | <span class="status-implemented">Implemented</span> | `ifc-model::ReverseIndex`, built on demand; records the attribute slot |
| Bounded graph traversal with cycle reports | <span class="status-implemented">Implemented</span> | `ifc-model::{depth_first, breadth_first, find_cycle}` with `Budget`/`Stop` |
| Dangling-reference detection | <span class="status-implemented">Implemented</span> | `Model::dangling_references` |
| **Schema-checked entity construction (authoring)** | <span class="status-implemented">Implemented</span> | `ifc-author::EntityBuilder`; facade feature `author`. See below. |

### Authoring

Applications that *generate* IFC name attributes; the schema decides positions.

```rust
use ifc::EntityBuilder;                 // feature = "author"

let id = EntityBuilder::new(&schema, "IfcAnnotation")
    .text("GlobalId", "3vB2YO$MX4xv5uCqZZG05x")
    .text("Name", "Brandwand")
    .insert(&mut model)?;
```

Slot order comes from `Schema::attributes`, which returns inherited attributes
first — the ordering positional STEP records depend on. `IfcAnnotation` gets
seven slots because IFC4 declares seven, not because the caller counted.

Construction is refused, rather than silently accepted, for:

| Failure | Example |
| --- | --- |
| Unknown entity | `IfcAnnotaton` (typo) |
| Unknown attribute | `IfcAnnotation.RefLatitude` (belongs to `IfcSite`) |
| Attribute set twice | two `Name` calls, instead of a silent overwrite |
| Required attribute unset | `IfcAnnotation` with no `GlobalId` |
| Declared-type mismatch | a string where `IfcLengthMeasure` is declared |
| Scalar/aggregate confusion | a scalar where `LIST OF` is declared |
| Malformed GlobalId | not 22 characters in IFC's base-64 alphabet |

A refused build leaves the model untouched.

**What this is not.** WHERE rules, inverse attributes, uniqueness, and
cross-entity consistency need a whole model rather than one entity;
`ifc-validate` owns those. Value checking is deliberately permissive where a
declared type cannot be resolved — see
[ADR 0007](/adr/0007-authoring-is-a-schema-layer-not-a-model-layer) for why
authoring is a schema-layer concern and not a model-layer one.

`Model::push` remains public and unchecked, for the case where an application
must write an entity the schema does not declare.

### Spatial traversal

IFC stores no parent pointers: a wall does not name its storey, a relationship
entity names both ends. `ifc-spatial` reads those relationships into a tree.

```rust
use ifc::{SpatialKind, SpatialTree};   // feature = "spatial"

let tree = SpatialTree::build(&model);

for storey in tree.of_kind(SpatialKind::Storey) {
    for element in tree.elements_of(storey.id) {
        // every element placed directly on this storey
    }
}

tree.container_of(wall);        // which storey is this wall on?
tree.ancestors(storey.id);      // storey -> building -> site -> project
tree.elements_recursive(root);  // everything beneath a container
```

**The trap this closes.** The two relationships that build the tree disagree
about slot order — `IfcRelAggregates` puts the parent in slot 4,
`IfcRelContainedInSpatialStructure` puts it in slot 5. Reading one like the
other inverts containment silently. The constants are asserted against IFC2x3,
IFC4 and IFC4x3 in `ifc-spatial/tests/slot_layout.rs`; see
[ADR 0008](/adr/0008-fixed-slot-constants-for-stable-relationships).

**Real files, not the ideal shape.** Omitted sites, elements hung directly off
a building, duplicate storeys, relationships naming absent entities, and
containment cycles are all handled and, where they are defects, reported through
`orphans()` and `dangling()` rather than dropped. One corpus fixture turned out
to use `IfcRelAggregates` exclusively with **no** containment relationship at
all; that case is pinned in `tests/real_files.rs`.

**What this is not.** It reports what the file says and never rejects it —
cardinality and WHERE rules belong to `ifc-validate`. It groups elements; it
does not interpret their geometry or properties.

## Geometry

`ifc-geometry` is the one substantial domain crate. It resolves IFC units,
placements, profiles, and representation relationships, then lowers implemented
families into the neutral `axiolid-model` DAG.

### Representation-item lowering

The dispatcher keeps coverage as data so it is auditable from one table
(`ifc-geometry/src/lower/dispatch.rs`):

| Family | Status |
| --- | --- |
| `IfcExtrudedAreaSolid` | <span class="status-implemented">Implemented</span> |
| `IfcRevolvedAreaSolid` | <span class="status-implemented">Implemented</span> |
| `IfcBooleanResult` | <span class="status-implemented">Implemented</span> |
| `IfcBooleanClippingResult` | <span class="status-implemented">Implemented</span> |
| `IfcMappedItem` | <span class="status-implemented">Implemented</span> |
| `IfcFacetedBrep` | <span class="status-implemented">Implemented</span> |
| `IfcFacetedBrepWithVoids` | <span class="status-implemented">Implemented</span> |
| `IfcAdvancedBrep` | <span class="status-partial">Planned</span> — advanced B-rep topology lowering |
| `IfcTriangulatedFaceSet` | <span class="status-partial">Planned</span> — tessellated face-set lowering |
| `IfcPolygonalFaceSet` | <span class="status-partial">Planned</span> — polygonal face-set lowering |
| `IfcSweptDiskSolid` | <span class="status-partial">Planned</span> |
| `IfcSurfaceCurveSweptAreaSolid` | <span class="status-partial">Planned</span> |
| `IfcSectionedSpine` | <span class="status-partial">Planned</span> |
| `IfcHalfSpaceSolid` | <span class="status-partial">Planned</span> |
| `IfcCsgSolid` | <span class="status-partial">Planned</span> |

::: warning Tessellated geometry is not lowered yet
`IfcTriangulatedFaceSet` and `IfcPolygonalFaceSet` are the dominant body
representation in IFC4 exports from several major authoring tools. Until they
are lowered, those bodies return a typed `GeometryError::Unsupported`. Plan for
this when estimating coverage against real project files.
:::

Unimplemented families return a typed `GeometryError::Unsupported` naming the
source entity and the specific missing capability — never a panic, and never a
silently substituted approximate shape.

### Curves and placement

| Capability | Status | Module |
| --- | --- | --- |
| `IfcPolyline` | <span class="status-implemented">Implemented</span> | `curve/polyline.rs` |
| `IfcCircle`, `IfcEllipse` | <span class="status-implemented">Implemented</span> | `curve/conic.rs` |
| `IfcLine` | <span class="status-implemented">Implemented</span> | `curve/line.rs` |
| `IfcTrimmedCurve` | <span class="status-implemented">Implemented</span> | `curve/trimmed.rs` |
| `IfcCompositeCurve` | <span class="status-implemented">Implemented</span> | `curve/composite.rs` |
| `IfcBSplineCurve` | <span class="status-implemented">Implemented</span> | `curve/bspline.rs` (representation) |
| `IfcOffsetCurve2D/3D` | <span class="status-implemented">Implemented</span> | `curve/offset.rs` |
| `IfcAxis2Placement2D/3D` | <span class="status-implemented">Implemented</span> | `resource/placement.rs` |
| `IfcCartesianTransformationOperator*` | <span class="status-implemented">Implemented</span> | `resource/operator.rs` |
| Unit resolution (SI, conversion-based) | <span class="status-implemented">Implemented</span> | `units.rs` |

::: warning Reading a curve is not lowering a curve
Every row above means the crate can **read** that entity's attributes into a
typed view. **No curve is lowered into the neutral geometry graph today**:
`lower/curve.rs` is a three-line placeholder, and
`lower_representation_item` dispatches no curve family, so a top-level
`IfcPolyline` in a representation returns `Unsupported`.

Polyline geometry *is* consumed in one place — as an arbitrary profile outline
inside a swept solid (`lower/profile.rs`). That is the only path where 2D curve
data reaches the graph.
:::

Curve *representation* is not an evaluator claim. Tessellating a B-spline is
Axiolid's concern, not this crate's — see
[the Axiolid boundary](/architecture/axiolid-boundary).

### Representation selection

`select_shape_representation` picks the representation a **viewer** should draw.
It prefers `Body`, then `Facetation`, then an unnamed representation, and
deliberately **refuses** `Axis` and `FootPrint` so a 2D curve never silently
replaces a solid.

Applications that *want* the 2D representations must select them directly; there
is currently no supported inverse selector. See the
[2D approval-plan use case](/use-cases/2d-approval-plans).

### Representation context

| Capability | Status | Evidence |
| --- | --- | --- |
| `IfcGeometricRepresentationContext` | <span class="status-absent">Absent</span> | zero occurrences in the workspace |
| `IfcGeometricRepresentationSubContext` | <span class="status-absent">Absent</span> | zero occurrences |
| `TargetView` (`PLAN_VIEW`, `MODEL_VIEW`, ...) | <span class="status-absent">Absent</span> | zero occurrences |

This is a hard blocker for drawing production. A plan-view annotation must live
in a sub-context whose `TargetView` is `PLAN_VIEW`; there is currently no way to
read, construct, or address one. Anything written without it is not a
schema-correct drawing, even though it round-trips.

## Presentation, annotation, and external references

These are the areas most relevant to drawing production and document/approval
workflows. They are currently the least built.

| Entity / concept | Status | Note |
| --- | --- | --- |
| `IfcAnnotation` | <span class="status-absent">Absent</span> | No reader, writer, or view |
| `IfcTextLiteral`, `IfcTextLiteralWithExtent` | <span class="status-absent">Absent</span> | |
| `IfcAnnotationFillArea` | <span class="status-absent">Absent</span> | |
| `IfcCurveStyle`, `IfcFillAreaStyle` | <span class="status-scaffold">Scaffold</span> | `ifc-style` reserves the module names only |
| `IfcSurfaceStyle` and children | <span class="status-scaffold">Scaffold</span> | `ifc-style/src/surface_style/` |
| `IfcPresentationLayerAssignment` | <span class="status-scaffold">Scaffold</span> | `ifc-style/src/layer/` |
| `IfcStyledItem` | <span class="status-scaffold">Scaffold</span> | `ifc-style/src/assignment/styled_item.rs` |
| `IfcShapeRepresentation` | <span class="status-absent">Absent</span> | Generic `Representation` view exists; the subtype does not |
| `IfcGeometricRepresentationSubContext` | <span class="status-absent">Absent</span> | Required to author a `Plan`/`Annotation` context |
| `IfcLibraryReference` | <span class="status-absent">Absent</span> | `ifc-classification/src/library/reference.rs` is a placeholder |
| `IfcLibraryInformation` | <span class="status-absent">Absent</span> | Placeholder file only |
| `IfcRelAssociatesLibrary` | <span class="status-absent">Absent</span> | |
| `IfcExternalReference` and `IfcLibrarySelect` | <span class="status-absent">Absent</span> | |
| `IfcApproval` and the whole `IfcApprovalResource` schema | <span class="status-absent">Absent</span> | No crate owns it; not even a scaffold |
| `IfcClassificationReference` | <span class="status-scaffold">Scaffold</span> | `ifc-classification` |

Because `ifc-model` round-trips entities structurally, a file containing any of
the above **reads and re-exports without loss today**. What is missing is typed
interpretation and typed authoring, not data preservation.

## Explicit non-goals

- A CAD modelling kernel. Booleans, tessellation, and NURBS evaluation live in
  Axiolid or its providers.
- Rendering, drawing layout, or paper-space composition.
- Vendored ISO or buildingSMART schema payloads
  (the [contributing guide](/guide/contributing#standards-material)).
- Any C++ in the dependency graph.

## How to verify a claim on this page

1. Read the crate's public API and its `tests/` directory.
2. Read the crate's **PLAN.md** — it records implementation state, unlike
   **AGENTS.md** which records stable contracts.
3. Run the gate: `scripts/gate.sh`.
4. For geometry coverage, read `ifc-geometry/src/lower/dispatch.rs`; the
   `IMPLEMENTED` and `PLANNED` constants are asserted by tests.
