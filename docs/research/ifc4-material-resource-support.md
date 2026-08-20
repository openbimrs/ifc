# IFC4 MaterialResource support inventory

Status: implementation source note. Normative input is the local IFC4 ADD2 TC1
EXPRESS schema at `/mnt/backup/references/ifc-spec/ifc4-add2-tc1/IFC4.exp` and
the matching HTML/PSD distribution. Runtime builds do not depend on that tree.

## Declarations

Machine extraction found four defined/select types, 18 entity declarations, and
one function in the requested resource:

- `IfcCardinalPointReference`: positive integer; conventional points 1-19 are
  named by the API while other schema-valid positive values remain lossless.
- `IfcDirectionSenseEnum`: `POSITIVE | NEGATIVE`.
- `IfcLayerSetDirectionEnum`: `AXIS1 | AXIS2 | AXIS3`.
- `IfcMaterialSelect`: material definition, material list, or material usage.
- `IfcMlsTotalThickness`: sum of `LayerThickness` across a material layer set.

Flattened IFC4 positional projections:

| Entity | Slots interpreted |
|---|---|
| IfcMaterial | Name, Description, Category |
| IfcMaterialClassificationRelationship | MaterialClassifications, ClassifiedMaterial |
| IfcMaterialConstituent | Name, Description, Material, Fraction, Category |
| IfcMaterialConstituentSet | Name, Description, MaterialConstituents |
| IfcMaterialDefinition | abstract; represented by `MaterialDefinition` |
| IfcMaterialLayer | Material, LayerThickness, IsVentilated, Name, Description, Category, Priority |
| IfcMaterialLayerSet | MaterialLayers, LayerSetName, Description |
| IfcMaterialLayerSetUsage | ForLayerSet, LayerSetDirection, DirectionSense, OffsetFromReferenceLine, ReferenceExtent |
| IfcMaterialLayerWithOffsets | layer slots plus OffsetDirection, OffsetValues |
| IfcMaterialList | Materials |
| IfcMaterialProfile | Name, Description, Material, Profile, Priority, Category |
| IfcMaterialProfileSet | Name, Description, MaterialProfiles, CompositeProfile |
| IfcMaterialProfileSetUsage | ForProfileSet, CardinalPoint, ReferenceExtent |
| IfcMaterialProfileSetUsageTapering | usage slots plus ForProfileEndSet, CardinalEndPoint |
| IfcMaterialProfileWithOffsets | profile slots plus OffsetValues |
| IfcMaterialProperties | Name, Description, Properties, Material |
| IfcMaterialRelationship | Name, Description, RelatingMaterial, RelatedMaterials, Expression |
| IfcMaterialUsageDefinition | abstract; represented by `MaterialUsageDefinition` |

All typed accessors reject wrong value variants, malformed scalar/nested
aggregates, empty required aggregates, missing required slots, non-finite
measures, and violated MaterialResource bounds. Typed-wrapper traversal is
iterative and capped, so malformed input cannot trigger unbounded recursion.

`IfcRelAssociatesMaterial`, declared outside MaterialResource, is also projected
because it is the normative `IfcMaterialSelect` assignment path for products and
types. Occurrence assignment takes precedence; type assignment is fallback, and
multiple matching relations remain ambiguous even when they point at the same
type. Resolution validates queried-object existence, immediate aggregate shape,
and the IFC4 concrete `IfcTypeObject` target inventory.

## Official PSD templates

The IFC4 corpus contains exactly the 14 requested `Pset_Material*` definitions.
All use `PSET_TYPEDRIVENOVERRIDE`; together they contain 78 top-level and 159
recursive property definitions. Nine target plain `IfcMaterial`; specialized
selectors add Concrete, Steel, and Wood templates.

`applicable_to` performs schema-safe entity-only matching. The separate
`applicable_to_category` adapter deliberately maps authored
`IfcMaterial.Category` to the PSD publication's slash qualifier. That mapping is
an explicit application policy, not an IFC `PredefinedType` rule.
`IfcMaterialProperties.Name` lookup remains exact and only returns property-set
templates whose applicability includes `IfcMaterial`; unrelated PSD/QTO names
are rejected.

The 14 names are pinned by `ifc/tests/material_template_inventory.rs`; the
application-layer join is pinned by `ifc/tests/material_templates.rs`.
Concrete, Steel, and Wood category queries cover the complete set without
bundling XML or the reference checkout.
