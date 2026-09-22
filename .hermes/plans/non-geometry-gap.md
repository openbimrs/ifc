# Authoring the last non-geometry gap

Working plan for the 40 entities that are unproven after `5c22a39` and are
**not** rooted in the geometry item tree. Geometry-rooted entities (20) are
out of scope by the user's instruction; see the exclusion rule below.

## Scope rule

Routing is by **schema supertype chain**, not by which crate happens to
mention the name. An entity is "geometry" when its chain reaches
`IfcGeometricRepresentationItem`, `IfcTopologicalRepresentationItem`,
`IfcRepresentationItem` or `IfcProfileDef`. That excludes 20 and leaves 40.

Two entities that grep placed in `ifc-style`/`ifc-geometry` are in scope
because their supertypes are not geometry: `IfcLightDistributionData` and
`IfcLightIntensityDistribution` are bare resource types, and
`IfcIndexedTriangleTextureMap` descends from `IfcPresentationItem`.
`IfcFillAreaStyleHatching`/`Tiles` and `IfcLightSourceGoniometric` *are*
geometry-rooted and are therefore excluded despite living in `ifc-style`.

## Clusters

| # | Cluster | Crate | Entities |
|---|---------|-------|----------|
| 1 | Facilities | `ifc-spatial` | `IfcMarineFacility`, `IfcRailway`, `IfcRoad`, `IfcBridgePart`, `IfcFacilityPartCommon`, `IfcMarinePart`, `IfcRailwayPart` |
| 2 | Structural reactions + conditions | `ifc-structural` | `IfcStructuralCurveReaction`, `IfcStructuralPointReaction`, `IfcStructuralSurfaceReaction`, `IfcStructuralResultGroup`, `IfcFailureConnectionCondition`, `IfcSlippageConnectionCondition`, `IfcRelConnectsWithEccentricity` |
| 3 | Space boundaries + path connections | `ifc-spatial` | `IfcRelSpaceBoundary`, `IfcRelSpaceBoundary1stLevel`, `IfcRelSpaceBoundary2ndLevel`, `IfcRelConnectsPathElements` |
| 4 | Groups | `ifc-systems` / `ifc-resource` | `IfcGroup`, `IfcInventory`, `IfcDistributionSystem` |
| 5 | Actors, addresses, resource relationships | `ifc-resource` | `IfcActorRole`, `IfcPostalAddress`, `IfcTelecomAddress`, `IfcOrganizationRelationship`, `IfcCurrencyRelationship`, `IfcDocumentInformationRelationship` |
| 6 | Property sets + quantities + units | `ifc-properties` | `IfcProfileProperties`, `IfcWindowPanelProperties`, `IfcQuantityNumber`, `IfcConversionBasedUnitWithOffset` |
| 7 | Presentation | `ifc-style` | `IfcSurfaceStyleRendering`, `IfcIndexedTriangleTextureMap`, `IfcLightDistributionData`, `IfcLightIntensityDistribution` |
| 8 | Remainder | various | `IfcShapeAspect`, `IfcTimePeriod`, `IfcTaskTimeRecurring`, `IfcWellKnownText`, `IfcGrid` |

## Method per cluster

Unchanged house playbook:

1. Read the `.exp` slots **and** WHERE rules for every entity in the cluster.
2. Grep the real helper API in the target crate before writing any call site.
   Never assume a helper name, a draft field, or an accessor exists.
3. Write the writer; encode rules by type where a missing field beats a check.
4. Tests: one staging test per entity plus one test per WHERE rule.
5. Mutation probes; every probe must be CAUGHT before the cluster ships.
6. `cargo fmt` -> `sync-capabilities.py` -> `sync-changelog.py` -> `gate.sh`.
7. Update the crate `AGENTS.md` boundary notes and `CHANGELOG.md`.
8. Commit per cluster. Re-measure coverage with the audit hook at the end.

## Measurement

Coverage is **proven by execution**, via the `authored-dump` hook on
`Transaction::create` (origin-tagged, so test-fixture `Model::push` calls do
not count). Baseline at the start of this work: **682 / 742**.

Do not quote a static-scan number. It is wrong in both directions: it cannot
see catalogue-row or tuple-dispatch writers, and it counts a name mentioned
in a validation list as authored.

## Pitfalls already paid for

- Attribute lookup is `eq_ignore_ascii_case`, so an IFC4/IFC4X3 spelling that
  differs only in case is **not** a bug. Only three true renames exist in the
  whole schema; all are handled.
- `IfcAxis2Placement` and friends are SELECTs, not supertypes. `is_a` rejects
  their members; check each concrete form or use `accepts_type`.
- A `Transaction` cannot be read back. To assert on a staged entity, commit
  first, or resolve the type from `tx.edits()`.
- `LIST [1:?]` means an empty list must stay `Value::Null`, not `()`.
