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
| Landed in a model by any path | 734 |
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

Every geometry declaration of IFC4 ADD2 TC1, by how this repository handles it (`ifc-geometry/data/ifc4-add2-tc1-geometry-support.tsv`):

| Status | Count |
| --- | ---: |
| `implemented` | 20 |
| `inventory` | 23 |
| `modeled-type` | 23 |
| `native-primitive` | 6 |
| `not-applicable` | 2 |
| `view-or-family` | 89 |
| **Total** | **163** |

Geometry WHERE rules (`ifc-geometry/data/ifc4-where-rules.tsv`):

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
| `nested-exact` | 31 |
| `non-shape` | 13 |
| `typed-refusal` | 4 |
| **Total** | **48** |

<details><summary>Every representation item (48)</summary>

| Entity | Disposition | Owner | Why |
| --- | --- | --- | --- |
| `IFCADVANCEDFACE` | `nested-exact` | `lower::brep` | face inside an advanced B-rep shell |
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
| `IFCFACESURFACE` | `nested-exact` | `lower::brep` | face with an authored carrier surface |
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

16 of 21 registered rules are evaluated; the rest are reported as unsupported rather than silently passed (`ifc-validate/src/where_rule/registry.rs`).

| Rule | Constrains | Evaluated | Why not |
| --- | --- | --- | --- |
| `global.IfcSingleProjectInstance` | (global) | yes |  |
| `global.UniqueGlobalId` | (global) | yes |  |
| `IfcRelDefinesByProperties.NoRelatedTypeObject` | `IfcRelDefinesByProperties` | yes |  |
| `IfcExternalReference.WR1` | `IfcExternalReference` | yes |  |
| `IfcRelSequence.WR1` | `IfcRelSequence` | yes |  |
| `IfcRelSequence.AvoidInconsistentSequence` | `IfcRelSequence` | yes |  |
| `IfcRelAggregates.NoSelfReference` | `IfcRelAggregates` | yes |  |
| `IfcRelNests.NoSelfReference` | `IfcRelNests` | yes |  |
| `IfcMaterialLayer.NormalizedPriority` | `IfcMaterialLayer` | yes |  |
| `IfcRelAssignsToActor.NoSelfReference` | `IfcRelAssignsToActor` | yes |  |
| `IfcRelAssignsToProcess.NoSelfReference` | `IfcRelAssignsToProcess` | yes |  |
| `IfcRelAssignsToProduct.NoSelfReference` | `IfcRelAssignsToProduct` | yes |  |
| `IfcRelAssignsToGroupByFactor.NoSelfReference` | `IfcRelAssignsToGroupByFactor` | yes |  |
| `IfcRelConnectsPathElements.NormalizedRelatingPriorities` | `IfcRelConnectsPathElements` | yes |  |
| `IfcRelConnectsPathElements.NormalizedRelatedPriorities` | `IfcRelConnectsPathElements` | yes |  |
| `IfcRelSpaceBoundary.CorrectPhysOrVirt` | `IfcRelSpaceBoundary` | yes |  |
| `IfcDocumentReference.WR1` | `IfcDocumentReference` | no | not implemented uniformly: IFC2X3 requires INVERSE relationship semantics, which validation does not derive |
| `IfcRepresentationContextSameWCS` | (global) | no | requires geometric evaluation, which validation does not perform |
| `IfcPolyLoop.WR21` | `IfcPolyLoop` | no | requires aggregate bounds, which the schema parser does not retain |
| `IfcPhysicalSimpleQuantity.WR21` | `IfcQuantityLength` | no | requires an EXPRESS expression evaluator |
| `IfcZone.WR1` | `IfcZone` | no | requires an EXPRESS expression evaluator |

<!-- COVERAGE:VALIDATION:END -->
