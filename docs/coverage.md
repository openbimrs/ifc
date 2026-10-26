# Coverage

What the IFC stack measurably covers. [Capabilities](/capabilities) says
what each crate does; this page counts it. Every table below is generated
from a machine-checked source in the repository and regenerated on every
change, so a number here is a measurement, not a claim.

## Authoring: every entity a writer can produce

The workspace test suite runs with an `ifc-model` hook that records every
entity type created through the authoring path (`Transaction::create`).
Entities that only arrive by parsing a file or loading a fixture do not
count. `scripts/gate.sh` measures this on every run and fails when the
result differs from the committed report.

<!-- COVERAGE:AUTHORED:BEGIN -->

| Measure | Entities |
| --- | ---: |
| Concrete entities in IFC4X3 ADD2 | 743 |
| Created by a writer in the test suite | 743 (100.0%) |
| Landed in a model by any path | 740 |
| Seen only through a codec or fixture, never written | 0 |
| Never produced at all | 0 |

<!-- COVERAGE:AUTHORED:END -->

## Geometry

`ifc-geometry` keeps a manifest of every geometry declaration in IFC4 ADD2
TC1, and a test asserts it against the schema, so nothing can be silently
missing. `implemented` lowers into the neutral geometry model. The other
statuses name where a declaration is handled instead: a typed view, a
modelled type, a native primitive, the inventory, or not applicable. The
per-family status is on the [capabilities page](/capabilities#representation-item-lowering).

<!-- COVERAGE:GEOMETRY:BEGIN -->

Every geometry declaration of IFC4 ADD2 TC1, by how this repository handles it (`crates/ifc-geometry/data/ifc4-add2-tc1-geometry-support.tsv`):

| Status | Count |
| --- | ---: |
| `implemented` | 20 |
| `inventory` | 23 |
| `modeled-type` | 23 |
| `native-primitive` | 6 |
| `not-applicable` | 2 |
| `view-or-family` | 89 |
| **Total** | **163** |

Geometry WHERE rules (`crates/ifc-geometry/data/ifc4-where-rules.tsv`):

| State | Count |
| --- | ---: |
| `implemented` | 95 |
| **Total** | **95** |

<!-- COVERAGE:GEOMETRY:END -->

### Representation items

Each concrete `IfcRepresentationItem` has one disposition:
- lowered exactly when nested in its owner;
- carrying no shape (presentation, owned by another crate);
- refused with a typed error.

<!-- COVERAGE:DISPOSITIONS:BEGIN -->

| Disposition | Count |
| --- | ---: |
| `nested-exact` | 29 |
| `non-shape` | 13 |
| `typed-refusal` | 4 |
| **Total** | **46** |

<details><summary>Every representation item (46)</summary>

| Entity | Disposition | Owner | Why |
| --- | --- | --- | --- |
| `IFCANNOTATIONFILLAREA` | `non-shape` | `ifc-style` | presentation fill whose boundary curves lower separately |
| `IFCAXIS1PLACEMENT` | `nested-exact` | `resource::placement` | placement input to exact geometry |
| `IFCAXIS2PLACEMENT2D` | `nested-exact` | `resource::placement` | placement input to exact geometry |
| `IFCAXIS2PLACEMENT3D` | `nested-exact` | `resource::placement` | placement input to exact geometry |
| `IFCCARTESIANPOINT` | `nested-exact` | `resource::point` | coordinate input to exact geometry |
| `IFCCARTESIANPOINTLIST2D` | `nested-exact` | `resource::point` | indexed coordinate input to curves and tessellation |
| `IFCCARTESIANPOINTLIST3D` | `nested-exact` | `resource::point` | indexed coordinate input to curves and tessellation |
| `IFCCARTESIANTRANSFORMATIONOPERATOR2D` | `nested-exact` | `resource::operator` | mapped-item transform input |
| `IFCCARTESIANTRANSFORMATIONOPERATOR2DNONUNIFORM` | `nested-exact` | `resource::operator` | mapped-item transform input |
| `IFCCARTESIANTRANSFORMATIONOPERATOR3D` | `nested-exact` | `resource::operator` | mapped-item transform input |
| `IFCCARTESIANTRANSFORMATIONOPERATOR3DNONUNIFORM` | `nested-exact` | `resource::operator` | mapped-item transform input |
| `IFCCLOSEDSHELL` | `nested-exact` | `lower::brep` | shell input to B-reps and surface models |
| `IFCCOMPOSITECURVESEGMENT` | `nested-exact` | `lower::curve` | ordered segment input to a composite curve |
| `IFCCONNECTEDFACESET` | `nested-exact` | `lower::brep` | face-set input to surface models |
| `IFCDIRECTION` | `nested-exact` | `resource::direction` | dimensionless orientation input validated before use |
| `IFCEDGE` | `nested-exact` | `lower::brep` | topological edge without carrier geometry |
| `IFCEDGECURVE` | `nested-exact` | `lower::brep` | topological edge with curve or p-curve geometry |
| `IFCEDGELOOP` | `nested-exact` | `lower::brep` | ordered edge-use input to a face bound |
| `IFCFACE` | `nested-exact` | `lower::brep` | face input to a connected face set |
| `IFCFACEBOUND` | `nested-exact` | `lower::brep` | oriented loop input to a face |
| `IFCFACEOUTERBOUND` | `nested-exact` | `lower::brep` | explicit outer-loop input to a face |
| `IFCFILLAREASTYLEHATCHING` | `non-shape` | `ifc-style` | presentation hatching rather than shape geometry |
| `IFCFILLAREASTYLETILES` | `non-shape` | `ifc-style` | presentation tiling rather than shape geometry |
| `IFCINDEXEDPOLYGONALFACE` | `nested-exact` | `lower::tessellated` | face index input to a polygonal face set |
| `IFCINDEXEDPOLYGONALFACEWITHVOIDS` | `nested-exact` | `lower::tessellated` | face-with-holes input to a polygonal face set |
| `IFCLIGHTSOURCEAMBIENT` | `non-shape` | `ifc-style` | lighting presentation state |
| `IFCLIGHTSOURCEDIRECTIONAL` | `non-shape` | `ifc-style` | lighting presentation state |
| `IFCLIGHTSOURCEGONIOMETRIC` | `non-shape` | `ifc-style` | lighting presentation state |
| `IFCLIGHTSOURCEPOSITIONAL` | `non-shape` | `ifc-style` | lighting presentation state |
| `IFCLIGHTSOURCESPOT` | `non-shape` | `ifc-style` | lighting presentation state |
| `IFCLOOP` | `typed-refusal` | `lower::brep` | generic loop has no concrete point or edge representation |
| `IFCOPENSHELL` | `nested-exact` | `lower::brep` | shell input to surface models |
| `IFCORIENTEDEDGE` | `nested-exact` | `lower::brep` | oriented edge use inside an edge loop |
| `IFCPATH` | `typed-refusal` | `lower::brep` | path topology is not a face-bound loop |
| `IFCPLANARBOX` | `non-shape` | `ifc-style` | presentation extent and placement |
| `IFCPLANAREXTENT` | `non-shape` | `ifc-style` | presentation extent rather than shape geometry |
| `IFCPOLYLOOP` | `nested-exact` | `lower::brep` | ordered point loop input to a face bound |
| `IFCREPARAMETRISEDCOMPOSITECURVESEGMENT` | `nested-exact` | `lower::curve` | composite segment with authored parameter length |
| `IFCSTYLEDITEM` | `non-shape` | `ifc-style` | style association without shape modification |
| `IFCSUBEDGE` | `nested-exact` | `lower::brep` | an edge carved from a parent edge; the subedge's own vertices ride the parent's curve, reached by walking ParentEdge |
| `IFCTEXTLITERAL` | `non-shape` | `ifc-style` | annotation text rather than shape geometry |
| `IFCTEXTLITERALWITHEXTENT` | `non-shape` | `ifc-style` | annotation text and extent rather than shape geometry |
| `IFCVECTOR` | `nested-exact` | `resource::direction` | direction plus magnitude input to exact curves and sweeps |
| `IFCVERTEX` | `typed-refusal` | `lower::brep` | generic vertex carries no point geometry |
| `IFCVERTEXLOOP` | `typed-refusal` | `lower::brep` | a single-vertex loop bounds zero area, so it contributes no face bound |
| `IFCVERTEXPOINT` | `nested-exact` | `lower::brep` | point-backed topological vertex |

</details>

<!-- COVERAGE:DISPOSITIONS:END -->

## Presentation

`ifc-style` classifies every appearance declaration of the schema it
touches:
- a strict borrowed view;
- a schema-checked value;
- a schema rule;
- structural passthrough.

<!-- COVERAGE:PRESENTATION:BEGIN -->

| Support | Count |
| --- | ---: |
| `SchemaRule` | 1 |
| `SchemaValue` | 24 |
| `StrictView` | 28 |
| `StructuralOnly` | 17 |
| **Total** | **70** |

<details><summary>Every appearance declaration (70)</summary>

| Declaration | Kind | Support |
| --- | --- | --- |
| `IfcBlobTexture` | Entity | `StrictView` |
| `IfcColour` | Type | `SchemaValue` |
| `IfcColourOrFactor` | Type | `StrictView` |
| `IfcColourRgb` | Entity | `StrictView` |
| `IfcColourRgbList` | Entity | `StructuralOnly` |
| `IfcColourSpecification` | Entity | `StructuralOnly` |
| `IfcCorrectFillAreaStyle` | Function | `SchemaRule` |
| `IfcCurveFontOrScaledCurveFontSelect` | Type | `SchemaValue` |
| `IfcCurveStyle` | Entity | `StrictView` |
| `IfcCurveStyleFont` | Entity | `StrictView` |
| `IfcCurveStyleFontAndScaling` | Entity | `StructuralOnly` |
| `IfcCurveStyleFontPattern` | Entity | `StrictView` |
| `IfcCurveStyleFontSelect` | Type | `SchemaValue` |
| `IfcDraughtingPreDefinedColour` | Entity | `StructuralOnly` |
| `IfcDraughtingPreDefinedCurveFont` | Entity | `StructuralOnly` |
| `IfcExternallyDefinedHatchStyle` | Entity | `StructuralOnly` |
| `IfcExternallyDefinedSurfaceStyle` | Entity | `StructuralOnly` |
| `IfcExternallyDefinedTextFont` | Entity | `StructuralOnly` |
| `IfcFillAreaStyle` | Entity | `StrictView` |
| `IfcFillAreaStyleHatching` | Entity | `StrictView` |
| `IfcFillAreaStyleTiles` | Entity | `StrictView` |
| `IfcFillStyleSelect` | Type | `SchemaValue` |
| `IfcFontStyle` | Type | `SchemaValue` |
| `IfcFontVariant` | Type | `SchemaValue` |
| `IfcFontWeight` | Type | `SchemaValue` |
| `IfcHatchLineDistanceSelect` | Type | `SchemaValue` |
| `IfcImageTexture` | Entity | `StrictView` |
| `IfcIndexedColourMap` | Entity | `StructuralOnly` |
| `IfcIndexedTextureMap` | Entity | `StrictView` |
| `IfcIndexedTriangleTextureMap` | Entity | `StrictView` |
| `IfcNullStyle` | Type | `SchemaValue` |
| `IfcPixelTexture` | Entity | `StrictView` |
| `IfcPreDefinedColour` | Entity | `StructuralOnly` |
| `IfcPreDefinedCurveFont` | Entity | `StructuralOnly` |
| `IfcPreDefinedItem` | Entity | `StructuralOnly` |
| `IfcPreDefinedTextFont` | Entity | `StructuralOnly` |
| `IfcPresentableText` | Type | `SchemaValue` |
| `IfcPresentationStyle` | Entity | `StructuralOnly` |
| `IfcPresentationStyleAssignment` | Entity | `StrictView` |
| `IfcPresentationStyleSelect` | Type | `SchemaValue` |
| `IfcReflectanceMethodEnum` | Type | `SchemaValue` |
| `IfcSizeSelect` | Type | `SchemaValue` |
| `IfcSpecularExponent` | Type | `SchemaValue` |
| `IfcSpecularHighlightSelect` | Type | `SchemaValue` |
| `IfcSpecularRoughness` | Type | `SchemaValue` |
| `IfcStyleAssignmentSelect` | Type | `SchemaValue` |
| `IfcStyledItem` | Entity | `StrictView` |
| `IfcSurfaceSide` | Type | `SchemaValue` |
| `IfcSurfaceStyle` | Entity | `StrictView` |
| `IfcSurfaceStyleElementSelect` | Type | `SchemaValue` |
| `IfcSurfaceStyleLighting` | Entity | `StrictView` |
| `IfcSurfaceStyleRefraction` | Entity | `StrictView` |
| `IfcSurfaceStyleRendering` | Entity | `StrictView` |
| `IfcSurfaceStyleShading` | Entity | `StrictView` |
| `IfcSurfaceStyleWithTextures` | Entity | `StrictView` |
| `IfcSurfaceTexture` | Entity | `StrictView` |
| `IfcTextAlignment` | Type | `SchemaValue` |
| `IfcTextDecoration` | Type | `SchemaValue` |
| `IfcTextFontName` | Type | `SchemaValue` |
| `IfcTextFontSelect` | Type | `SchemaValue` |
| `IfcTextStyle` | Entity | `StrictView` |
| `IfcTextStyleFontModel` | Entity | `StrictView` |
| `IfcTextStyleForDefinedFont` | Entity | `StructuralOnly` |
| `IfcTextStyleTextModel` | Entity | `StructuralOnly` |
| `IfcTextTransformation` | Type | `SchemaValue` |
| `IfcTextureCoordinate` | Entity | `StrictView` |
| `IfcTextureCoordinateGenerator` | Entity | `StrictView` |
| `IfcTextureMap` | Entity | `StructuralOnly` |
| `IfcTextureVertex` | Entity | `StrictView` |
| `IfcTextureVertexList` | Entity | `StrictView` |

</details>

<!-- COVERAGE:PRESENTATION:END -->

## Validation

`ifc-validate` registers every WHERE rule it knows about with an explicit
state. An unsupported rule is reported as unsupported, so a clean report
never means "the rules we did not implement passed".

<!-- COVERAGE:VALIDATION:BEGIN -->

20 of 26 registered rules are evaluated; the rest are reported as unsupported rather than silently passed (`crates/ifc-validate/src/where_rule/registry.rs`). A rule binds its declaring entity and every subtype, and runs only under the releases whose EXPRESS declares it under that id.

| Rule | Constrains | Releases | Evaluated | Why not |
| --- | --- | --- | --- | --- |
| `global.IfcSingleProjectInstance` | (global) | IFC2X3, IFC4, IFC4X3 | yes |  |
| `global.UniqueGlobalId` | (global) | IFC2X3, IFC4, IFC4X1, IFC4X2, IFC4X3 | yes |  |
| `IfcRelDefinesByProperties.NoRelatedTypeObject` | `IfcRelDefinesByProperties` and subtypes | IFC4, IFC4X3 | yes |  |
| `IfcExternalReference.WR1` | `IfcExternalReference` and subtypes | IFC2X3, IFC4, IFC4X3 | yes |  |
| `IfcRelSequence.WR1` | `IfcRelSequence` and subtypes | IFC2X3 | yes |  |
| `IfcRelSequence.AvoidInconsistentSequence` | `IfcRelSequence` and subtypes | IFC4, IFC4X3 | yes |  |
| `IfcRelAggregates.NoSelfReference` | `IfcRelAggregates` and subtypes | IFC4, IFC4X3 | yes |  |
| `IfcRelNests.NoSelfReference` | `IfcRelNests` and subtypes | IFC4, IFC4X3 | yes |  |
| `IfcMaterialLayer.NormalizedPriority` | `IfcMaterialLayer` and subtypes | IFC4, IFC4X3 | yes |  |
| `IfcRelAssignsToActor.NoSelfReference` | `IfcRelAssignsToActor` and subtypes | IFC4, IFC4X3 | yes |  |
| `IfcRelAssignsToProcess.NoSelfReference` | `IfcRelAssignsToProcess` and subtypes | IFC4, IFC4X3 | yes |  |
| `IfcRelAssignsToProduct.NoSelfReference` | `IfcRelAssignsToProduct` and subtypes | IFC4, IFC4X3 | yes |  |
| `IfcRelAssignsToGroup.NoSelfReference` | `IfcRelAssignsToGroup` and subtypes | IFC4, IFC4X3 | yes |  |
| `IfcRelConnectsPathElements.NormalizedRelatingPriorities` | `IfcRelConnectsPathElements` and subtypes | IFC4, IFC4X3 | yes |  |
| `IfcRelConnectsPathElements.NormalizedRelatedPriorities` | `IfcRelConnectsPathElements` and subtypes | IFC4, IFC4X3 | yes |  |
| `IfcRelSpaceBoundary.CorrectPhysOrVirt` | `IfcRelSpaceBoundary` and subtypes | IFC4, IFC4X3 | yes |  |
| `IfcObject.UniquePropertySetNames` | `IfcObject` and subtypes | IFC4, IFC4X3 | yes |  |
| `IfcTypeObject.UniquePropertySetNames` | `IfcTypeObject` and subtypes | IFC4, IFC4X3 | yes |  |
| `IfcTypeProduct.WR41` | `IfcTypeProduct` and subtypes | IFC2X3 | yes |  |
| `IfcTypeProduct.ApplicableOccurrence` | `IfcTypeProduct` and subtypes | IFC4, IFC4X3 | yes |  |
| `IfcDocumentReference.WR1` | `IfcDocumentReference` and subtypes | IFC2X3, IFC4, IFC4X3 | no | not implemented uniformly: IFC2X3 requires INVERSE relationship semantics, which validation does not derive |
| `IfcRepresentationContextSameWCS` | (global) | IFC2X3, IFC4, IFC4X3 | no | requires geometric evaluation, which validation does not perform |
| `IfcPolyLoop.WR21` | `IfcPolyLoop` and subtypes | IFC2X3 | no | requires an EXPRESS expression evaluator |
| `IfcPolyLoop.AllPointsSameDim` | `IfcPolyLoop` and subtypes | IFC4, IFC4X3 | no | requires an EXPRESS expression evaluator |
| `IfcQuantityLength.WR21` | `IfcQuantityLength` and subtypes | IFC2X3, IFC4, IFC4X3 | no | requires an EXPRESS expression evaluator |
| `IfcZone.WR1` | `IfcZone` and subtypes | IFC2X3, IFC4, IFC4X3 | no | requires an EXPRESS expression evaluator |

<!-- COVERAGE:VALIDATION:END -->

### EXPRESS rules

Given the release's own `.exp` text, `ifc_validate::express::ExpressRules`
evaluates every entity WHERE rule (inherited ones included), every type
WHERE rule and every global rule of the release, with the FUNCTIONs they
call, and the hand-written rules above with the same id are not run. The
schemas are CC BY-ND 4.0, so the rule text is supplied at run time and never
bundled. The CLI (`validate --express FILE.exp`) and the JavaScript, Python,
C and .NET bindings take it the same way, and refuse text that does not
match the file's release.

<!-- COVERAGE:EXPRESS:BEGIN -->

Declarations the evaluator can run, of those each release declares (`crates/ifc-validate/data/express-coverage.tsv`, asserted against the normative schemas by `crates/ifc-validate/tests/express_rules/schemas.rs`). A declaration is evaluable when neither it nor a function it calls needs a construct the evaluator refuses; at run time a rule may still stop at a budget or at data it cannot read, which is reported per instance.

| Release | Entity WHERE rules | Type WHERE rules | Global rules | Functions | Derived attributes | Not evaluable |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| IFC2X3_TC1 | 339 / 339 | 24 / 24 | 2 / 2 | 38 / 38 | 55 / 55 | none |
| IFC4_ADD2_TC1 | 652 / 652 | 25 / 25 | 2 / 2 | 47 / 47 | 62 / 62 | none |
| IFC4X1_FINAL | 658 / 658 | 25 / 25 | 2 / 2 | 47 / 47 | 62 / 62 | none |
| IFC4X2_FINAL | 672 / 672 | 25 / 25 | 2 / 2 | 47 / 47 | 62 / 62 | none |
| IFC4X3_ADD2 | 752 / 752 | 25 / 25 | 2 / 2 | 48 / 48 | 60 / 60 | none |

<!-- COVERAGE:EXPRESS:END -->

Compared with IfcOpenShell 0.8.5's `express_rules` mode instance by
instance, on this repository's fixtures, IfcOpenShell's own 138 rule
fixtures and 37 validation fixtures, and the 48 IFC4 ADD2 TC1 annex E
examples, the two agree on 202 violations and on every INVERSE cardinality
finding of the fixtures. The differences:

- `'CENTER'` as an `IfcBoxAlignment` is a violation here: EXPRESS compares
  strings exactly and the rule lists lower-case values. IfcOpenShell
  compares case-blind and names the file a pass.
- Rule strings naming a type in mixed case (`'IFC4X3_ADD2.IfcMappedItem'`,
  in IFC4X1, IFC4X2 and IFC4X3 ADD2) are read upper-cased here, as
  IfcOpenShell reads them; read exactly, `TYPEOF` could never match them.
- Where IfcOpenShell's generated rule code fails on the data (nested
  lists: `IfcPositiveInteger.WR1` on three annex E tessellations,
  `IfcRationalBSplineSurfaceWithKnots.WeightValuesGreaterZero`), it
  reports a violation; the rule holds here.
- IfcOpenShell's generated rule code reads the literal `'by layer'` as
  `'bylayer'`, so `IfcCurveStyle.WR11` fails on its own fixture named a
  pass; it holds here.
- A rule this evaluator cannot decide on the data, such as a measure
  written as a string, is an evaluation error here and a violation there.
- An UNKNOWN or `?` result passes silently in IfcOpenShell and is reported
  as indeterminate here.
- `TYPEOF` here includes the SELECT types a value belongs to, so
  `'IFC4.IFCCOLOUR' IN TYPEOF(x)` holds for an `IfcColourRgb`; IfcOpenShell
  lists entity and defined types only.
