# Roadmap

Ordered by what unblocks the most downstream work. Each item states the gap, the
entities involved, and where the code would live, so it can be picked up without
re-deriving the analysis.

Status vocabulary matches the [capability matrix](/capabilities).

## Now — foundations that unblock applications

### R1. Typed authoring layer — done

Delivered as `ifc-author` (facade feature `author`). Entities are built by
attribute name; slot positions come from `Schema::attributes`. Construction is
refused for unknown entities and attributes, duplicate sets, missing required
attributes, declared-type and aggregate mismatches, and malformed GlobalIds.

Remaining under `AUTHOR` in the crate's **PLAN.md**: editing an entity already in a
model (blocked on `MODEL-MUT` transactional apply) and a decision on whether
`IfcOwnerHistory` is authored here or injected by an application service.

### R2. Relationship and spatial traversal — done

Delivered as `ifc-spatial` (facade feature `spatial`), on top of two new
`ifc-model` primitives: `ReverseIndex` (built on demand, records the attribute
slot) and bounded `depth_first`/`breadth_first`/`find_cycle` walks with explicit
budgets and cycle reports.

`SpatialTree` assembles project → site → building → storey → element and
tolerates real files: omitted levels, elements on the building, duplicate
storeys, dangling references, containment cycles. Defects are reported through
`orphans()` and `dangling()` instead of being dropped.

Slot layouts are constants rather than schema lookups, because the two
relationships that build the tree disagree about slot order and the failure mode
is silent inversion. See
[ADR 0008](/adr/0008-fixed-slot-constants-for-stable-relationships).

Remaining under `SPATIAL`: reusing the reverse index for inverse queries
(`relation::naming` currently rescans), and grouping properties by container
(blocked — `ifc-properties` is scaffold).

## Next — presentation and external references

### R3. Representation contexts and `IfcShapeRepresentation` — done

`RepresentationContext` reads `IfcGeometricRepresentationContext` and its
sub-context: identifier, type, parent, target scale and a typed `TargetView`
(`PLAN_VIEW`, `MODEL_VIEW`, ... ; unknown constants preserved rather than
flattened). `plan_contexts` finds the sub-contexts a drawing is authored into.

`select_plan_representation` is the inverse of `select_shape_representation`:
it prefers a drawable identifier inside an explicit `PLAN_VIEW` context, then
any `Plan`/`Annotation`/`FootPrint`/`Axis` match, and returns `None` for a
solid-only or box-only product rather than handing back something to draw flat.
Context and identifier are intersected: a bounding box in a plan context is
still not drawable geometry.

The trap this closes is DERIVED inheritance. A sub-context redeclares six
inherited attributes and real files write them as `*`, meaning "read this from
my parent". Accessors taking `&model` resolve the chain; reading the slot
directly yields the marker and loses the project's precision and placement. See
[ADR 0009](/adr/0009-derived-attributes-resolve-through-the-parent-context).

Not included: authoring helpers for plan sub-contexts (constructible today via
`EntityBuilder`, just without a dedicated builder), and deriving plan geometry
from solids — that needs sectioning, tracked as R9b/R10.

### R4. `ifc-style` — presentation appearance

**Gap.** The whole crate is scaffold: `IfcCurveStyle`, `IfcFillAreaStyle` and
its hatching/tiles children, `IfcSurfaceStyle` and its shading/lighting/
rendering/refraction children, `IfcColourRgb`, `IfcStyledItem`,
`IfcPresentationLayerAssignment` and `IfcPresentationLayerWithStyle`, and the
texture family.

**Why.** Without it, geometry can be produced but not given line weights, line
types, colours, hatches, or layers — i.e. it cannot be made into a drawing.

### R5. Annotation entities

**Gap.** `IfcAnnotation`, `IfcTextLiteral`, `IfcTextLiteralWithExtent`,
`IfcAnnotationFillArea`, and `IfcGeometricCurveSet` handling for annotation
purposes are absent. No text means no room labels, no dimension text, no
legends, no title-block content.

**Note.** IFC2x3's `IfcDimensionCurve` family is correctly absent — it was
removed in IFC4. The IFC4 replacement is `IfcAnnotation` plus `ObjectType`,
property sets, and an `Annotation` representation context, which makes R3 a
prerequisite.

**Home.** Needs a decision: extend `ifc-style`, or add a dedicated
presentation/annotation crate. Annotation is arguably product-level rather than
style-level, so a separate crate is likely cleaner.

### R6. External references and libraries

**Gap.** `IfcExternalReference` (the abstract base), `IfcLibraryReference`,
`IfcLibraryInformation`, `IfcLibrarySelect`, and `IfcRelAssociatesLibrary` are
absent. `ifc-classification/src/library/reference.rs` and `information.rs` exist
as empty placeholders — the intended home is already reserved.

**Why it matters.** Library references are how a symbol set, a component
catalogue, or a classification source stays portable and vendor-neutral rather
than proprietary to one application.

**Scope note.** `IfcExternalReference` is the shared base for library,
classification, and document references, so implementing it once unblocks three
families at the same time.

### R7. Approvals

**Gap.** The entire `IfcApprovalResource` schema is absent — `IfcApproval`,
`IfcApprovalRelationship`, `IfcRelAssociatesApproval`. **No crate owns it**, not
even as a scaffold.

**Design note.** `IfcApproval` is *not* rooted — it has no `GlobalId` — so it
needs different identity handling from the `IfcRel*` entities. Do not assume the
`IfcRoot` attribute layout.

**Home.** Needs a new crate or module; the natural pairing is with documents and
external references (R6), since approval, document, and library references share
the association pattern.

## Later — geometry coverage

### R8. Tessellated face sets

`IfcTriangulatedFaceSet` and `IfcPolygonalFaceSet` are declared `PLANNED` in the
dispatcher. They are the dominant body representation in IFC4 exports from
several major authoring tools, so this gap disproportionately affects real-file
coverage.

### R9. Remaining representation-item families

`IfcAdvancedBrep`, `IfcSweptDiskSolid`, `IfcSurfaceCurveSweptAreaSolid`,
`IfcSectionedSpine`, `IfcHalfSpaceSolid`, `IfcCsgSolid` — each declared in
`PLANNED` with a stated reason.

### R9b. Curve lowering

**Gap.** Curve *readers* exist for every 2D family, but `lower/curve.rs` is a
three-line placeholder and `lower_representation_item` dispatches no curve
family. A top-level `IfcPolyline` in a representation returns `Unsupported`.
Polyline data reaches the graph only as a swept-solid profile outline.

**Shape.** Implement `lower::curve` and add the curve families to the
dispatcher's `IMPLEMENTED` table, mirroring how profiles are handled.

**Why it matters.** Any 2D drawing pipeline needs curves as first-class graph
output, not only as profile inputs. Prerequisite for R3 being useful.

### R10. Plan derivation from 3D

Sectioning solids at a cut height to derive plan geometry. This is a **kernel**
capability, not an IFC one, and no plane-section operation exists in Axiolid at
the pinned revision. Tracking it here because applications ask this repository
for it; the work itself belongs upstream.

## Domain crates awaiting implementation

`ifc-properties`, `ifc-validate`, `ifc-schedule`, `ifc-resource`,
`ifc-systems`, `ifc-structural`, `ifc-georef`, `ifc-alignment` are all
scaffolds. Each has a **PLAN.md** recording its intended scope. Priority among
them is demand-driven; open an issue describing the use case.

## Contributing

Items R1 and R2 are the highest-leverage and the most self-contained. See
[contributing](/guide/contributing).
