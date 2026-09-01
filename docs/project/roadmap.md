# Roadmap

Ordered by what unblocks the most downstream work. The roadmap is the curated
product narrative; the repository's nested **PLAN.md** files remain the complete
engineering backlog and evidence log. Only tasks promoted to the public
[Ready for contributors](https://github.com/orgs/openbimrs/projects/1/views/3)
view should be treated as implementation-ready.

Status vocabulary matches the [capability matrix](/capabilities). Capability
claims below name the implementation or executable evidence that proves them.

## Delivered foundations

### R0. Declared-schema validation coverage — done, bounded

IFC2X3 TC1, IFC4 ADD2 TC1, and IFC4X3 ADD2 each have independently generated
bundled structural tables. `validate_declared` selects only the file's declared
version; all 38 committed fixtures are now audited (31 model-level clean, 7
expected-invalid with observable errors, 0 schema skips). Two raw-header-only
defects are normalized by the codec before model validation.

Validation covers structural/reference/type facts plus eight selected native
checks represented by nine version-labelled rule IDs. This is not full EXPRESS
conformance: aggregate bounds, arbitrary `WHERE` expressions, INVERSE derivation,
and geometry-dependent semantics remain explicitly unsupported and reported.

### R1. Typed authoring layer — done

Delivered as `ifc-author` (facade feature `author`). Entities are built and
edited by attribute name; slot positions come from `Schema::attributes`.
Construction and projected edits refuse unknown entities and attributes,
duplicate sets, missing required attributes, declared-type and aggregate
mismatches, malformed GlobalIds, and malformed existing arity. Edits stage
through `ifc-model::Transaction`, so dangling references and stale revisions
leave the model untouched.

Remaining under `AUTHOR` in the crate's **PLAN.md**: a decision on whether
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

`SPATIAL-INV` is implemented: callers with repeated inverse lookups build one
borrowed `RelationshipIndex`, which reuses `ifc-model::ReverseIndex` while
preserving tolerant relationship decoding. Grouping implemented `ifc-properties`
views by container remains an L4 orchestration seam.

## Presentation and external references

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

**Implemented.** Schema-resolved borrowed views cover `IfcCurveStyle`,
`IfcFillAreaStyle` and its hatch/tile children, `IfcSurfaceStyle` and its
shading/lighting/rendering/refraction children, `IfcColourRgb`, `IfcStyledItem`,
presentation layers, and the texture family. Selected core style graphs have
transaction-staged writers, and style resolution preserves deterministic
direct-over-layer precedence.

**Remaining boundary.** These APIs interpret and author IFC presentation data;
they do not render it or compose a drawing.

### R5. Annotation entities

**Implemented.** `ifc-style` provides strict borrowed views and bounded
transaction-staged writers for `IfcAnnotation`, `IfcTextLiteral`,
`IfcTextLiteralWithExtent`, and `IfcAnnotationFillArea`. Annotation type,
text path, and box alignment use bounded schema vocabularies.

**Note.** IFC2x3's `IfcDimensionCurve` family is correctly absent — it was
removed in IFC4. The IFC4 replacement is `IfcAnnotation` plus `ObjectType`,
property sets, and an `Annotation` representation context, which makes R3 a
prerequisite.

**Remaining boundary.** Placement, composition, rendering, and general
annotation geometry lowering remain application/geometry concerns.

### R6. External references and libraries

**Implemented.** `ifc-classification` now exposes borrowed IFC4 views for
classification systems/references, document information/references, library
information/references, all three `IfcRelAssociates*` families, and the generic
`IfcExternalReferenceRelationship`. Bounded hierarchy lookup reports cycles,
dangling/wrong-kind references, and budget exhaustion. Effective classification
lookup returns occurrence and type associations separately. Matching
transactional helpers author all ten concrete records after validating each draft.

**Why it matters.** Library references are how a symbol set, a component
catalogue, or a classification source stays portable and vendor-neutral rather
than proprietary to one application.

**Boundary.** Approval and constraint resource relationships live in dedicated
sibling domains rather than the classification namespace.

### R7. Approvals and constraints

**Implemented.** `ifc-approval` owns `IfcApproval`,
`IfcApprovalRelationship`, `IfcResourceApprovalRelationship`, and
`IfcRelAssociatesApproval`. `ifc-constraint` owns concrete metrics/objectives,
`IfcResourceConstraintRelationship`, and `IfcRelAssociatesConstraint`. Both
provide strict borrowed views, deterministic direct queries, and typed
transaction-staged authoring. A facade/STEP test proves classification, approval,
and constraint projections join through stable `EntityId` values.

**Boundary.** Approval state is preserved but not treated as authorization,
signature, workflow, or policy. Constraints are preserved but not evaluated as
compliance rules, formulas, reference paths, tables, or time series.

## Geometry coverage

### R8. Tessellated face sets — done

`IfcTriangulatedFaceSet` and `IfcPolygonalFaceSet` lower through
`ifc-geometry/src/lower/tessellated.rs` into neutral mesh values. Authored
triangles, n-gons, and polygonal holes are preserved without inferred topology
or forced triangulation. Unit and fixture tests plus the corpus census pin the
behaviour; both families are in the dispatcher's `IMPLEMENTED` table.

### R9. Tracked representation-item families — done for the listed set

The formerly planned `IfcAdvancedBrep`, `IfcSweptDiskSolid`,
`IfcSurfaceCurveSweptAreaSolid`, `IfcSectionedSpine`, half-space, and CSG
families now appear in `ifc-geometry/src/lower/dispatch.rs::IMPLEMENTED` and
have focused lowering tests. Advanced B-reps preserve shared topology and exact
curve/surface handles; half spaces retain their cutting-side semantics; swept
and CSG families remain exact neutral operations rather than eager meshes.

**Remaining boundary.** The dispatcher still declares
`IfcArbitraryOpenProfileDef` unsupported because the neutral profile model
represents closed contours only. Broader exact profile and surface coverage is
tracked by `GEOM-PROFILE`, `GEOM-SURFACE`, and `LOW-EXACT`, not by the completed
families above.

### R9b. Curve lowering — partial

`ifc-geometry/src/lower/curve.rs` now emits exact neutral nodes for lines,
polylines, circles, trimmed curves, composite curves, and explicit-knot
B-splines, including rational weights. The implementation preserves curve
parameter units, segment sense, transition codes, closure, and trim selectors;
it is exercised directly and through swept-solid fixtures.

**Remaining gap.** The representation-item dispatcher now lowers the supported
curve families as top-level items as well as directrices and members of a
geometric collection. Other concrete curve families remain typed `Unsupported`
results. `GEOM-CURVE` owns completing the family census without approximation.

### R10. Plan derivation from 3D

Sectioning solids at a cut height to derive plan geometry. This is a **kernel**
capability, not an IFC one, and no plane-section operation exists in Axiolid at
the pinned revision. Tracking it here because applications ask this repository
for it; the work itself belongs upstream.

## Partial domain slices

`ifc-resource` now has a tested IFC4 construction-resource vertical slice:
six concrete occurrence kinds, authored resource-time values, allocation lookup,
budgeted authored-order composition, and transaction-staged creation of selected
records and relationships. Actor/inventory/resource-type semantics and IFC2X3 or
IFC4X3 profiles remain open; it is therefore partial rather than generally
implemented.

`ifc-georef` and `ifc-alignment` also have tested partial vertical slices: IFC4
project-to-map/CRS resolution in the former, and IFC4X3 segment parameters plus
selected exact curve output in the latter. `ifc-classification`, `ifc-approval`,
`ifc-constraint`, `ifc-properties`, `ifc-style`, `ifc-structural`, `ifc-validate`,
`ifc-schedule`, `ifc-systems`, and the bounded `ifc-cost` contract are implemented;
`ifc-resource`, `ifc-geometry`, `ifc-georef`, and `ifc-alignment` are partial. Each crate's **PLAN.md** records
the remaining scope. Priority among unfinished slices is demand-driven; open an
issue describing the use case.

## Contributing

Use the public [Ready for contributors](https://github.com/orgs/openbimrs/projects/1/views/3)
view rather than choosing an arbitrary unchecked task. Promoted issues name one
stable **PLAN.md** task ID, its current blockers, scope, and required proof. This
keeps the detailed backlog in Git while GitHub owns assignment and discussion.

Ideas and unresolved ownership questions, such as the home for approvals, start
in [repository Discussions](https://github.com/openbimrs/ifc/discussions).
Concrete bugs and feature requests use the issue forms. See the full
[contributing guide](/guide/contributing).
