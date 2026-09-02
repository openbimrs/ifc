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

Generated from the source tree by `scripts/sync-capabilities.py`, not estimated.
"Stub files" counts source files of twelve lines or fewer — the placeholder
shape described above. `scripts/gate.sh` fails if these numbers drift from the
code.

<!-- CAPABILITIES:CENSUS:BEGIN -->

| Crate | Source LOC | Files | Stub files | Test files | Status |
| --- | ---: | ---: | ---: | ---: | --- |
| `ifc-geometry` | 26,181 | 90 | 5 | 32 | <span class="status-partial">Partial</span> |
| `ifc-structural` | 3,397 | 33 | 14 | 12 | <span class="status-implemented">Implemented</span> |
| `ifc-style` | 3,322 | 31 | 0 | 5 | <span class="status-implemented">Implemented</span> |
| `ifc-properties` | 3,119 | 30 | 14 | 3 | <span class="status-implemented">Implemented</span> |
| `ifc-template-catalog` | 2,705 | 29 | 3 | 10 | <span class="status-implemented">Implemented</span> |
| `ifc-material` | 2,398 | 24 | 0 | 8 | <span class="status-implemented">Implemented</span> |
| `ifc-validate` | 2,249 | 23 | 0 | 2 | <span class="status-implemented">Implemented</span> |
| `ifc-classification` | 2,203 | 20 | 4 | 3 | <span class="status-implemented">Implemented</span> |
| `ifc-model` | 2,083 | 25 | 5 | 10 | <span class="status-implemented">Implemented</span> |
| `ifc-cost` | 2,001 | 16 | 0 | 2 | <span class="status-implemented">Implemented</span> |
| `ifc-resource` | 1,827 | 29 | 18 | 4 | <span class="status-partial">Partial</span> |
| `ifc-schedule` | 1,691 | 24 | 14 | 1 | <span class="status-implemented">Implemented</span> |
| `ifc-systems` | 1,586 | 20 | 5 | 2 | <span class="status-implemented">Implemented</span> |
| `ifc-constraint` | 1,230 | 6 | 0 | 1 | <span class="status-implemented">Implemented</span> |
| `ifc-alignment` | 1,095 | 26 | 16 | 2 | <span class="status-partial">Partial</span> |
| `ifc-xml` | 1,045 | 6 | 0 | 3 | <span class="status-implemented">Implemented</span> |
| `ifc-schema` | 1,038 | 10 | 4 | 2 | <span class="status-implemented">Implemented</span> |
| `ifc-approval` | 915 | 5 | 0 | 1 | <span class="status-implemented">Implemented</span> |
| `openbim-ifc` | 900 | 6 | 0 | 12 | <span class="status-implemented">Implemented</span> |
| `ifc-author` | 729 | 8 | 3 | 3 | <span class="status-implemented">Implemented</span> |
| `ifc-spatial` | 615 | 8 | 1 | 4 | <span class="status-implemented">Implemented</span> |
| `ifc-georef` | 582 | 17 | 12 | 1 | <span class="status-partial">Partial</span> |
| `ifc-step` | 516 | 5 | 0 | 3 | <span class="status-implemented">Implemented</span> |

<!-- CAPABILITIES:CENSUS:END -->

<!-- CAPABILITIES:SCAFFOLDCOUNT:BEGIN -->

0 of 23 crates are scaffolds.

<!-- CAPABILITIES:SCAFFOLDCOUNT:END -->
They exist because the layering decision
(see [ADR 0005](/adr/0005-scaffold-modules-declare-ownership))
prefers declaring the intended home of a domain up front over discovering it
later, but they must never be mistaken for working code.

## Core: model, codecs, schema

| Capability | Status | Evidence |
| --- | --- | --- |
| Entity graph with positional attributes | <span class="status-implemented">Implemented</span> | `ifc-model::Model` |
| Round-trip of entities the build does not understand | <span class="status-implemented">Implemented</span> | `openbim-ifc/tests/costing_roundtrip.rs` (runs with no domain crate compiled) |
| STEP (`.ifc`) read and write | <span class="status-implemented">Implemented</span> | `ifc-step`; deterministic model order, finite scalar safeguards, generic syntax delegated to `openbim-step` |
| ifcXML read and write | <span class="status-implemented">Implemented</span> | `ifc-xml`; explicit strict IFC4 ADD2 TC1 namespace profile and path-rich typed diagnostics; compatibility dialect is not claimed as generic XSD conformance |
| IFC-JSON | <span class="status-absent">Absent</span> | Would be a third `Codec` impl; no crate exists |
| EXPRESS schema metadata, subtype queries | <span class="status-implemented">Implemented</span> | `ifc-schema` |
| GlobalId base-64 encode/decode | <span class="status-implemented">Implemented</span> | `ifc-model::guid` |
| Spatial containment tree traversal | <span class="status-implemented">Implemented</span> | `ifc-spatial::SpatialTree`; facade feature `spatial`. See below. |
| Objectified relationship traversal | <span class="status-partial">Partial</span> | `ifc-spatial::relation` reads `IfcRelAggregates`, `IfcRelContainedInSpatialStructure`, `IfcRelNests`. Other `IfcRel*` families are not interpreted. |
| Distribution systems, ports and connectivity | <span class="status-implemented">Implemented</span> | `ifc-systems` reads systems and membership, ports through both `IfcRelNests` and the legacy `IfcRelConnectsPortToElement`, the connection network, flow roles and direction, zones with their `WR1` membership rule, spatial containment vs referencing, and direction-aware `upstream`/`downstream` queries. Relationship-only: no geometry is read, so a geometry-free file still yields a full network. |
| Cost items, rates and rollups | <span class="status-implemented">Implemented</span> | `ifc-cost` reads `IfcCostItem` nesting, `IfcCostValue` component trees with arithmetic operators, and totals a cost tree. Currencies are compared, never converted: a rollup mixing EUR and USD is refused. Typed drafts stage selected IFC4 values, items, schedules, nesting, and schedule assignments atomically. |
| Work schedules, tasks and sequencing | <span class="status-implemented">Implemented</span> | `ifc-schedule` reads `IfcWorkPlan`/`IfcWorkSchedule`, `IfcTask` with `IfcTaskTime`, `IfcRelSequence` with signed lag, work calendars and events, and produces a deterministic execution order. Cycles report the offending path. |
| Transactional authoring | <span class="status-implemented">Implemented</span> | `ifc-model::Transaction` stages structural edits, validates them against the projected end state, and applies them as a unit. A removal that would orphan a surviving reference is refused, as is a commit against a model whose revision moved since the transaction opened. `ifc-model` remains schema-agnostic; typed staging helpers live in domain crates, including classification, documents, libraries, materials, quantities, cost, style, structural, and resources. |
| Quantity authoring | <span class="status-implemented">Implemented</span> | `ifc-properties` stages quantity writes onto a caller-owned transaction, so a takeoff spanning many elements lands atomically. The declared measure type is preserved on every write. |
| Schema validation | <span class="status-implemented">Implemented</span> | `ifc-validate` checks references, required slots, aggregate shape, entity types, abstract instantiation, scalar forms and `STRING(n) FIXED` widths against the exact schema the file declares (IFC2x3 TC1, IFC4 ADD2 TC1, or IFC4X3 ADD2). Nine registered rule IDs cover eight native checks under a hard findings-storage cap, including external-reference identity, sequence endpoints, decomposition self-reference and material priority. Aggregate bounds, arbitrary EXPRESS `WHERE` expressions, INVERSE semantics and other known gaps remain explicitly unsupported and reported; a clean report never implies full EXPRESS conformance. |
| Property sets and every property value family | <span class="status-implemented">Implemented</span> | `ifc-properties` reads single, enumerated, bounded, list, table, reference and complex properties. The declared measure type (`IfcLengthMeasure` and friends) is retained, because it is the only statement of what a bare number means. |
| Occurrence/type property precedence | <span class="status-implemented">Implemented</span> | An occurrence property set overrides a same-named set inherited from the object's type. The shadowed type set is kept, so a checker can explain why an effective value differs from the type default. |
| Quantities and unit resolution | <span class="status-implemented">Implemented</span> | Simple and complex quantities, SI prefixes carried as exact decimal exponents, conversion-based and derived units. `WR21` (unit matches quantity kind) and `WR22` (non-negative value) breaches are reported; `ifcopenshell.validate` checks neither. Quantities are read as authored assertions and never computed from geometry. |
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

### Construction resources

With facade feature `resource`, `ifc-resource` selects IFC4 from the model header
and exposes borrowed views for labor, equipment, crew, construction material,
construction product, and subcontract occurrences. It resolves authored
`IfcResourceTime`, validates and queries `IfcRelAssignsToResource` including
its authored `RelatedObjectsType` category constraint, and walks resource
composition through `IfcRelNests` with authored order plus explicit
cycle, multiple-parent, depth, and node failures. Selected records and
relationships can be created through schema-checked transaction-staged drafts;
they reject duplicate `GlobalId` values and cycle/second-parent creation before
staging. See the
[construction-resource guide](/guide/resources).

This is a partial, deliberately bounded profile. It does not level or schedule
resources, calculate costs/durations/quantities, interpret calendars, or claim
actor, inventory, construction-resource-type, IFC2X3, or IFC4X3 support.

## Geometry

`ifc-geometry` is the one substantial domain crate. It resolves IFC units,
placements, profiles, and representation relationships, then lowers implemented
families into the neutral `axiolid-model` DAG.

### Representation-item lowering

The dispatcher keeps coverage as data so it is auditable from one table
(`ifc-geometry/src/lower/dispatch.rs`):

<!-- CAPABILITIES:GEOMETRY:BEGIN -->

| Family | Status |
| --- | --- |
| `IfcExtrudedAreaSolid` | <span class="status-implemented">Implemented</span> |
| `IfcRevolvedAreaSolid` | <span class="status-implemented">Implemented</span> |
| `IfcBooleanResult` | <span class="status-implemented">Implemented</span> |
| `IfcBooleanClippingResult` | <span class="status-implemented">Implemented</span> |
| `IfcMappedItem` | <span class="status-implemented">Implemented</span> |
| `IfcFacetedBrep` | <span class="status-implemented">Implemented</span> |
| `IfcFacetedBrepWithVoids` | <span class="status-implemented">Implemented</span> |
| `IfcAdvancedBrep` | <span class="status-implemented">Implemented</span> |
| `IfcAdvancedBrepWithVoids` | <span class="status-implemented">Implemented</span> |
| `IfcHalfSpaceSolid` | <span class="status-implemented">Implemented</span> |
| `IfcBoxedHalfSpace` | <span class="status-implemented">Implemented</span> |
| `IfcTriangulatedFaceSet` | <span class="status-implemented">Implemented</span> |
| `IfcPolygonalFaceSet` | <span class="status-implemented">Implemented</span> |
| `IfcCsgSolid` | <span class="status-implemented">Implemented</span> |
| `IfcSweptDiskSolid` | <span class="status-implemented">Implemented</span> |
| `IfcSweptDiskSolidPolygonal` | <span class="status-implemented">Implemented</span> |
| `IfcSurfaceCurveSweptAreaSolid` | <span class="status-implemented">Implemented</span> |
| `IfcBlock` | <span class="status-implemented">Implemented</span> |
| `IfcSphere` | <span class="status-implemented">Implemented</span> |
| `IfcRightCircularCylinder` | <span class="status-implemented">Implemented</span> |
| `IfcRightCircularCone` | <span class="status-implemented">Implemented</span> |
| `IfcRectangularPyramid` | <span class="status-implemented">Implemented</span> |
| `IfcBoundingBox` | <span class="status-implemented">Implemented</span> |
| `IfcExtrudedAreaSolidTapered` | <span class="status-implemented">Implemented</span> |
| `IfcRevolvedAreaSolidTapered` | <span class="status-implemented">Implemented</span> |
| `IfcFixedReferenceSweptAreaSolid` | <span class="status-implemented">Implemented</span> |
| `IfcSectionedSpine` | <span class="status-implemented">Implemented</span> |
| `IfcShellBasedSurfaceModel` | <span class="status-implemented">Implemented</span> |
| `IfcFaceBasedSurfaceModel` | <span class="status-implemented">Implemented</span> |
| `IfcGeometricSet` | <span class="status-implemented">Implemented</span> |
| `IfcGeometricCurveSet` | <span class="status-implemented">Implemented</span> |
| `IfcLine` | <span class="status-implemented">Implemented</span> |
| `IfcCircle` | <span class="status-implemented">Implemented</span> |
| `IfcEllipse` | <span class="status-implemented">Implemented</span> |
| `IfcPolyline` | <span class="status-implemented">Implemented</span> |
| `IfcIndexedPolyCurve` | <span class="status-implemented">Implemented</span> |
| `IfcCompositeCurve` | <span class="status-implemented">Implemented</span> |
| `IfcCompositeCurveOnSurface` | <span class="status-implemented">Implemented</span> |
| `IfcBoundaryCurve` | <span class="status-implemented">Implemented</span> |
| `IfcOuterBoundaryCurve` | <span class="status-implemented">Implemented</span> |
| `IfcTrimmedCurve` | <span class="status-implemented">Implemented</span> |
| `IfcOffsetCurve2D` | <span class="status-implemented">Implemented</span> |
| `IfcOffsetCurve3D` | <span class="status-implemented">Implemented</span> |
| `IfcPcurve` | <span class="status-implemented">Implemented</span> |
| `IfcSurfaceCurve` | <span class="status-implemented">Implemented</span> |
| `IfcIntersectionCurve` | <span class="status-implemented">Implemented</span> |
| `IfcSeamCurve` | <span class="status-implemented">Implemented</span> |
| `IfcBSplineCurveWithKnots` | <span class="status-implemented">Implemented</span> |
| `IfcRationalBSplineCurveWithKnots` | <span class="status-implemented">Implemented</span> |
| `IfcPlane` | <span class="status-implemented">Implemented</span> |
| `IfcCylindricalSurface` | <span class="status-implemented">Implemented</span> |
| `IfcSphericalSurface` | <span class="status-implemented">Implemented</span> |
| `IfcToroidalSurface` | <span class="status-implemented">Implemented</span> |
| `IfcSurfaceOfLinearExtrusion` | <span class="status-implemented">Implemented</span> |
| `IfcSurfaceOfRevolution` | <span class="status-implemented">Implemented</span> |
| `IfcRectangularTrimmedSurface` | <span class="status-implemented">Implemented</span> |
| `IfcCurveBoundedPlane` | <span class="status-implemented">Implemented</span> |
| `IfcCurveBoundedSurface` | <span class="status-implemented">Implemented</span> |
| `IfcBSplineSurfaceWithKnots` | <span class="status-implemented">Implemented</span> |
| `IfcRationalBSplineSurfaceWithKnots` | <span class="status-implemented">Implemented</span> |
| `IfcPolygonalBoundedHalfSpace` | <span class="status-partial">Planned</span> — polygonal boundary cannot be discarded; exact bounded-half-space support is required |
| `IfcPointOnCurve` | <span class="status-partial">Planned</span> — exact point evaluation reference is not yet represented |
| `IfcPointOnSurface` | <span class="status-partial">Planned</span> — exact surface-parameter point is not yet represented |

<!-- CAPABILITIES:GEOMETRY:END -->

::: tip Coverage below is generated
The table above is derived from `ifc-geometry/src/lower/dispatch.rs` by
`scripts/sync-capabilities.py`, and `scripts/gate.sh` fails when this page and
that source disagree. It cannot drift from the code without breaking the build.
:::

Unimplemented families return a typed `GeometryError::Unsupported` naming the
source entity and the specific missing capability — never a panic, and never a
silently substituted approximate shape.

### Profile lowering

Swept solids reference a profile, so profile coverage bounds how much of a real
model lowers. Steel sections carry their fillet radii, edge radii and flange
slopes into the neutral model rather than being reduced to an outline, because
a section without them has the wrong area and the wrong section modulus.

<!-- CAPABILITIES:PROFILE:BEGIN -->

| Profile family | Status |
| --- | --- |
| `IfcArbitraryClosedProfileDef` | <span class="status-implemented">Implemented</span> |
| `IfcArbitraryOpenProfileDef` | <span class="status-implemented">Implemented</span> |
| `IfcArbitraryProfileDefWithVoids` | <span class="status-implemented">Implemented</span> |
| `IfcAsymmetricIShapeProfileDef` | <span class="status-implemented">Implemented</span> |
| `IfcCenterLineProfileDef` | <span class="status-implemented">Implemented</span> |
| `IfcCircleHollowProfileDef` | <span class="status-implemented">Implemented</span> |
| `IfcCircleProfileDef` | <span class="status-implemented">Implemented</span> |
| `IfcCompositeProfileDef` | <span class="status-implemented">Implemented</span> |
| `IfcCShapeProfileDef` | <span class="status-implemented">Implemented</span> |
| `IfcDerivedProfileDef` | <span class="status-implemented">Implemented</span> |
| `IfcEllipseProfileDef` | <span class="status-implemented">Implemented</span> |
| `IfcIShapeProfileDef` | <span class="status-implemented">Implemented</span> |
| `IfcLShapeProfileDef` | <span class="status-implemented">Implemented</span> |
| `IfcMirroredProfileDef` | <span class="status-implemented">Implemented</span> |
| `IfcRectangleHollowProfileDef` | <span class="status-implemented">Implemented</span> |
| `IfcRectangleProfileDef` | <span class="status-implemented">Implemented</span> |
| `IfcRoundedRectangleProfileDef` | <span class="status-implemented">Implemented</span> |
| `IfcTShapeProfileDef` | <span class="status-implemented">Implemented</span> |
| `IfcTrapeziumProfileDef` | <span class="status-implemented">Implemented</span> |
| `IfcUShapeProfileDef` | <span class="status-implemented">Implemented</span> |
| `IfcZShapeProfileDef` | <span class="status-implemented">Implemented</span> |
| `IfcProfileDef` | <span class="status-partial">Planned</span> — generic profile declaration carries no concrete geometry to lower |

<!-- CAPABILITIES:PROFILE:END -->

`IfcArbitraryOpenProfileDef` is implemented through the explicit
`lower_open_profile_node` authored-path API. It is not an area profile and is
therefore still refused when an extrusion or other area-based solid requests
`lower_profile`.

### Curves and placement

| Capability | Status | Module |
| --- | --- | --- |
| `IfcPolyline`, `IfcIndexedPolyCurve` | <span class="status-implemented">Implemented</span> | exact line and three-point arc segments |
| `IfcCircle`, `IfcEllipse` | <span class="status-implemented">Implemented</span> | `curve/conic.rs` |
| `IfcLine` | <span class="status-implemented">Implemented</span> | `curve/line.rs` |
| `IfcTrimmedCurve` | <span class="status-implemented">Implemented</span> | `curve/trimmed.rs` |
| `IfcCompositeCurve` | <span class="status-implemented">Implemented</span> | `curve/composite.rs` |
| Convention-only `IfcBSplineCurve` | <span class="status-partial">Partial</span> | typed view only; lowering does not invent absent knots |
| `IfcBSplineCurveWithKnots`, `IfcRationalBSplineCurveWithKnots` | <span class="status-implemented">Implemented</span> | `curve/bspline.rs` representation + exact neutral lowering |
| `IfcOffsetCurve2D/3D` | <span class="status-implemented">Implemented</span> | `curve/offset.rs` |
| `IfcPcurve`, `IfcSurfaceCurve`, `IfcIntersectionCurve` | <span class="status-partial">Partial</span> | exact supported forms; typed refusal otherwise |
| `IfcAxis2Placement2D/3D` | <span class="status-implemented">Implemented</span> | `resource/placement.rs` |
| `IfcCartesianTransformationOperator*` | <span class="status-implemented">Implemented</span> | `resource/operator.rs` |
| Unit resolution (SI, conversion-based) | <span class="status-implemented">Implemented</span> | `units.rs` |

::: info Curves lower in nested and curve-representation paths
`lower/curve.rs` lowers polylines/indexed poly-curves, conics, lines, trims,
composites, offsets and surface-associated curves, plus explicit-knot
polynomial/rational B-splines into the neutral graph. Convention-only base
splines remain unsupported. Curves reach this lowerer through a sweep directrix,
a surface boundary, a B-rep edge, or an `IfcGeometricCurveSet` member.

Bare curves are dispatch targets for `Curve2D`, `Curve3D`, and plan
representations. Selection remains explicit: callers choose Body or Plan; a plan
curve never silently replaces a body.
:::

### Explicit-knot surfaces

| Capability | Status | Module |
| --- | --- | --- |
| Convention-only `IfcBSplineSurface` | <span class="status-partial">Partial</span> | typed view only; lowering ...[truncated]

### Representation selection

Two selectors, deliberately disagreeing:

```rust
use ifc::{select_shape_representation, select_plan_representation};

select_shape_representation(&model, wall)?;  // -> the Body   (3D viewer)
select_plan_representation(&model, wall)?;   // -> the FootPrint (drawing)
```

`select_shape_representation` prefers `Body`, then `Facetation`, then an
unnamed representation, and **refuses** `Axis`/`FootPrint` so a 2D curve never
silently replaces a solid.

`select_plan_representation` is its inverse. It prefers, in order:

1. a `PLAN_IDENTIFIERS` match **inside** a `PLAN_VIEW` sub-context — drawable
   geometry the author explicitly targeted at a plan;
2. otherwise the best `PLAN_IDENTIFIERS` match in any context: `Plan`,
   `Annotation`, `FootPrint`, `Axis`.

The two rules are intersected, not ordered. Authorial intent selects *between*
drawable candidates; it does not make a bounding box drawable. ArchiCAD writes
`Box`/`BoundingBox` representations inside `PLAN_VIEW` sub-contexts, so a
context-first rule returns those boxes and never reaches the identifier list.

It returns `None` for a solid-only or box-only product. That is the honest answer, not a
failure: deriving a plan from a solid requires sectioning, which this library
does not do — see [R9b/R10 on the roadmap](/project/roadmap).

### Representation context

| Capability | Status | Evidence |
| --- | --- | --- |
| `IfcGeometricRepresentationContext` | <span class="status-implemented">Implemented</span> | `RepresentationContext`; identifier, type, precision, placement |
| `IfcGeometricRepresentationSubContext` | <span class="status-implemented">Implemented</span> | parent, target scale, target view |
| `TargetView` (`PLAN_VIEW`, `MODEL_VIEW`, ...) | <span class="status-implemented">Implemented</span> | typed enum; unknown constants preserved, not flattened |
| Authoring a plan sub-context | <span class="status-partial">Partial</span> | constructible via `EntityBuilder`; no dedicated helper |

```rust
use ifc::plan_contexts;

for context in plan_contexts(&model) {
    context.target_scale();          // Some(0.01) for 1:100
    context.precision(&model);       // inherited from the parent context
}
```

**The `*` trap this closes.** A sub-context redeclares six inherited attributes
as DERIVED, and real files write them as `*`:

```text
IFCGEOMETRICREPRESENTATIONSUBCONTEXT('Body','Model',*,*,*,*,#1,$,.MODEL_VIEW.,$)
```

`*` is not `$` — it means "read this from my parent". Accessors that take
`&model` (`precision`, `world_coordinate_system`, `coordinate_space_dimension`,
`true_north`) walk `ParentContext` to resolve it. A consumer reading the slot
directly gets the marker and loses the project's precision and placement. See
[ADR 0009](/adr/0009-derived-attributes-resolve-through-the-parent-context).

Slot positions are asserted against IFC2x3, IFC4 and IFC4x3 in
`ifc-geometry/tests/context_slots.rs`, including that the sub-context still
inherits exactly six attributes — the off-by-six that would read `TargetScale`
as the target view.

## Presentation, annotation, and external references

These are the areas most relevant to drawing production and document/approval
workflows. Presentation and annotation now have schema-resolved typed views and
bounded transaction-staged authoring; rendering, drawing layout, and the
approval resource remain outside the current domain contracts. External
classification/document/library references have typed read/query/authoring
coverage.

| Entity / concept | Status | Note |
| --- | --- | --- |
| `IfcAnnotation` | <span class="status-implemented">Implemented</span> | Strict borrowed view and transaction-staged authoring; IFC4X3 `PredefinedType` remains version-conditional |
| `IfcTextLiteral`, `IfcTextLiteralWithExtent` | <span class="status-implemented">Implemented</span> | Strict literal/placement/path/extent views and transaction-staged authoring |
| `IfcAnnotationFillArea` | <span class="status-implemented">Implemented</span> | Type-checked outer/inner boundary views and transaction-staged authoring |
| `IfcCurveStyle`, `IfcFillAreaStyle` | <span class="status-implemented">Implemented</span> | Schema-resolved curve/font/fill/hatch/tile views; no rendering claim |
| `IfcSurfaceStyle` and children | <span class="status-implemented">Implemented</span> | RGB/factor, shading, rendering, lighting, refraction, and texture views; core surface graph authoring |
| `IfcPresentationLayerAssignment` | <span class="status-implemented">Implemented</span> | Layer membership/style/visibility view and styled-layer authoring |
| `IfcStyledItem` | <span class="status-implemented">Implemented</span> | Strict direct assignment view/writer and deterministic direct-over-layer resolution; IFC2x3 wrappers are explicit |
| `IfcShapeRepresentation` | <span class="status-absent">Absent</span> | Generic `Representation` view exists; the subtype does not |
| `IfcGeometricRepresentationSubContext` | <span class="status-partial">Partial</span> | Strict inherited view and plan-context query exist; authoring uses generic `EntityBuilder`, not a dedicated helper |
| `IfcLibraryReference` | <span class="status-implemented">Implemented</span> | Strict borrowed view and transaction-staged authoring |
| `IfcLibraryInformation` | <span class="status-implemented">Implemented</span> | Strict borrowed view and transaction-staged authoring |
| `IfcRelAssociatesLibrary` | <span class="status-implemented">Implemented</span> | Deterministic object lookup and transaction-staged authoring |
| `IfcExternalReference` and `IfcLibrarySelect` | <span class="status-implemented">Implemented</span> | Inherited fields are exposed by concrete views; select targets are type-checked |
| `IfcExternalReferenceRelationship` | <span class="status-implemented">Implemented</span> | `ifc-classification` owns strict selected projections, deterministic lookup, and transaction-staged authoring |
| `IfcApproval` and selected approval relationships | <span class="status-implemented">Implemented</span> | `ifc-approval` owns bounded IFC4 projections, queries, and transaction-staged authoring; the whole `IfcApprovalResource` schema is not claimed |
| `IfcMetric`, `IfcObjective`, and selected constraint relationships | <span class="status-implemented">Implemented</span> | `ifc-constraint` preserves typed metric values and stages bounded IFC4 authoring without evaluating constraints |
| `IfcClassificationReference` | <span class="status-implemented">Implemented</span> | Bounded hierarchy, explicit occurrence/type sources, and authoring |

Because `ifc-model` round-trips entities structurally, every row above reads and
re-exports without loss. The status distinguishes that baseline from typed
interpretation and authoring; only rows marked implemented expose those domain
contracts.

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
