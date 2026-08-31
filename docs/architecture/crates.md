# Crate map

Twenty-one crates in one workspace. Status uses the vocabulary defined on the
[capabilities page](/capabilities): **Implemented** means executable behaviour
with tests; **Scaffold** means reserved module names with no behaviour.

## L0 — record core

### `ifc-model`
The entity graph. Owns `Model`, `Entity`, `Value`, `Header`, `EntityId`, the
`Codec` trait, and IFC's base-64 `GlobalId` encoding. Schema-, codec-, and
domain-agnostic by construction.

Implemented: storage and ordering, the type and reverse-reference indices,
dangling-reference detection, bounded graph traversal with cycle reports, GUID
encode/decode, and header handling. Remaining module-level ownership seams are
tracked in the capability matrix.

## L1 — schema and codecs

### `ifc-schema`
EXPRESS schema metadata as queryable data: entity declarations, supertype
chains, subtype queries, conformant XML names. Useful for validating an
authoring layer's attribute arity against the declared schema.

### `ifc-step`
The ISO 10303-21 physical file codec (`.ifc`). Generic STEP syntax and EXPRESS
parsing are delegated to the `openbim-step` crate; this crate retains thin
model, schema-version, and validation adapters.

### `ifc-xml`
The ifcXML codec (`.ifcxml`). Implements the same `Codec` trait, which is what
makes format conversion a two-line operation.

## L2 — geometry bridges

### `ifc-geometry`
By far the largest crate. Typed and family-shared borrowed views over the three
IFC geometry resource schemas, plus lowering into the neutral `axiolid-model`
DAG. Resolves units, placements, profiles, and representation relationships.

Seven representation-item families lower today; eight more are declared
`PLANNED` with a stated reason. Unimplemented families return a typed
`Unsupported` error rather than panicking or substituting approximate geometry.

### `ifc-alignment`, `ifc-georef`
Scaffold. Reserved for alignment (horizontal, vertical, cant) and
georeferencing (CRS, map conversion).

## L2 — domain views

### `ifc-material`
Implemented. Material definitions, layer/profile/constituent sets, and
template applicability.

### `ifc-template-catalog`
Implemented. Versioned PSD and QTO template data derived from the published
IFC4 ADD2 TC1 definitions.

### `ifc-cost`
Partial. Real modules for cost items, quantities, rollup, and schedule views.

### `ifc-properties`
Implemented. Borrowed property-set, quantity, unit, template, and standard
library projections, including typed quantity authoring staged on a caller-owned
transaction. `ifc-template-catalog` carries the published catalog data.

### `ifc-style`
Implemented bounded presentation domain. It provides schema-resolved borrowed
views for curve, fill, text, surface, texture, styled-item, layer, and named
annotation entities; selected core graphs have transaction-staged authoring and
deterministic direct-over-layer resolution. It is not a renderer or drawing
layout engine.

### `ifc-classification`
Implemented bounded IFC4 classification, document, and library views, hierarchy
queries, association lookup, and transaction-staged authoring for nine concrete
records. Abstract external-reference resources are not claimed generally.

### `ifc-schedule`, `ifc-systems`
Implemented bounded schedule/task sequencing and relationship-only distribution
system/port/connectivity views.

### `ifc-resource`
Scaffold. Reserved for construction resources: labour, equipment, consumed or
capacity-bearing materials/products, usage, time, cost and process allocation.

### `ifc-structural`
Implemented bounded structural-analysis domain. Schema-resolved borrowed views
cover analysis/load/result groups, analytical members and connections, actions,
core static-load values and selected relationships across IFC2X3, IFC4 and
IFC4X3. Canonical enum membership and exposed entity-specific `WHERE`/measure
constraints are enforced; selected analysis-model and load authoring is
transaction-staged. It is not a solver, FEM layer, geometry evaluator or
computed-results engine.

### `ifc-validate`
Implemented schema-structural validation against the exact supported schema,
plus nine bounded native rule IDs. It does not claim arbitrary EXPRESS `WHERE`
evaluation or general `INVERSE` semantics.

### `ifc-spatial`
Implemented. Containment and objectified relationship traversal: reads
`IfcRelAggregates`, `IfcRelContainedInSpatialStructure` and `IfcRelNests` into
a project/site/building/storey/element tree, reporting orphaned containers and
dangling references rather than asserting the canonical hierarchy.

Depends on `ifc-model` only. The slot layouts it needs are stable across
IFC2x3/IFC4/IFC4x3 and are asserted against all three, so it does not pay for a
schema parse. See [ADR 0008](/adr/0008-fixed-slot-constants-for-stable-relationships).

### `ifc-author`
Implemented. Schema-checked construction: build an entity by naming its
attributes and let `ifc-schema` resolve the STEP slot positions. Refuses unknown
entities and attributes, duplicate sets, missing required attributes,
declared-type and aggregate mismatches, and malformed GlobalIds.

It is the write-side counterpart to the domain views, and the reason authoring
is not in `ifc-model`: the record core is L0 and must not depend on the schema
tables. See [ADR 0007](/adr/0007-authoring-is-a-schema-layer-not-a-model-layer).

## L3 — facade

### `openbim-ifc`
The published crate. Re-exports the model unconditionally and gates every codec
and domain behind a cargo feature. Library target is named `ifc`.

Its test suite is where the cross-cutting invariants live: `thin_build.rs`
proves a minimal build compiles no domain code, and `costing_roundtrip.rs`
proves unknown-domain data survives a round trip.

## Not in this repository

- **Axiolid** — the format-neutral geometry kernel this crate lowers into.
  Separate project; see [the boundary page](/architecture/axiolid-boundary).
- **`openbim-step`** — generic ISO 10303-21 / EXPRESS substrate.
- **`openbimrs/openbim`** — the integration superproject, which pins a verified
  commit of this repository as `packages/ifc`.
- Sibling standards live in their own repositories under the same organisation:
  IDS, BCF, ICDD, IDM, LOIN, GAEB, MMC, and others.
