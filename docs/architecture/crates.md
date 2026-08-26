# Crate map

Nineteen crates in one workspace. Status uses the vocabulary defined on the
[capabilities page](/capabilities): **Implemented** means executable behaviour
with tests; **Scaffold** means reserved module names with no behaviour.

## L0 — record core

### `ifc-model`
The entity graph. Owns `Model`, `Entity`, `Value`, `Header`, `EntityId`, the
`Codec` trait, and IFC's base-64 `GlobalId` encoding. Schema-, codec-, and
domain-agnostic by construction.

Implemented: storage and ordering, the type index, dangling-reference detection,
GUID encode/decode, header handling.
Scaffold: `spatial`, `relation`, `traverse`, the reverse-reference index,
`provenance`.

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
Scaffold. Reserved for property sets, quantities, units, and templates. Note
that `ifc-template-catalog` already carries the catalog data.

### `ifc-style`
Scaffold. Reserved for presentation appearance: curve styles, fill-area styles,
surface styles, textures, styled items, and layer assignment. This is the crate
an annotation or drawing-production workflow needs, and it has no behaviour yet.

### `ifc-classification`
Scaffold. Reserved for classification systems and references, and — importantly
— for `IfcLibraryReference` / `IfcLibraryInformation` under `src/library/`.

### `ifc-schedule`, `ifc-resource`, `ifc-systems`, `ifc-structural`
Scaffold. Reserved for work schedules and sequencing; construction resources,
actors and inventory; distribution systems and connectivity; and structural
analysis members, actions and connections.

### `ifc-validate`
Scaffold. Reserved for where-rule evaluation, type checking, structural checks,
and report shaping.

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
