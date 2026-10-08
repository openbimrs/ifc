<script setup>
import facts from '../.vitepress/data/facts.json'
</script>

# Crate map

The workspace is {{ facts.crates.total }} crates, versioned and released
independently. Dependencies point down the tiers described in
[System design](/architecture/#dependency-tiers):

- **L0, the record core.** `ifc-model` owns the entity graph and the `Codec`
  trait, and knows no schema, codec or domain.
- **L1, schema and codecs.** `ifc-schema` holds the bundled EXPRESS tables;
  `ifc-step` and `ifc-xml` implement `Codec`. Codecs never import domain
  semantics.
- **L2, views, validation, authoring and geometry.** Each domain crate is a
  borrowed projection over the model, and sibling domains never depend on
  one another. `ifc-geometry`, `ifc-georef` and `ifc-alignment` lower
  geometry into the neutral Axiolid model.
- **L3, the facade.** `openbim-ifc` re-exports the crates above behind cargo
  features, with a thin default.
- **L4, the bindings.** JavaScript, Python and C wrap the facade through a
  shared host-independent core.

The map below is generated from each crate's manifest, so the dependency
column is what cargo resolves, not what anyone remembers. Each crate's
[reference page](/reference/) has its overview, features and latest
changes; the [capability matrix](/capabilities) says what each implements.

<!-- CRATES:MAP:BEGIN -->

### Facade

| Crate | Status | Depends on | Description |
| --- | --- | --- | --- |
| [`openbim-ifc`](/reference/crates/openbim-ifc) | <span class="status-implemented">Implemented</span> | the core, domain and geometry crates, each behind a feature | Facade for the openBIM IFC crates: pick codecs and domains as features. |

### Model, codecs, schema, authoring and validation

| Crate | Status | Depends on | Description |
| --- | --- | --- | --- |
| [`ifc-author`](/reference/crates/ifc-author) | <span class="status-implemented">Implemented</span> | [`ifc-model`](/reference/crates/ifc-model), [`ifc-schema`](/reference/crates/ifc-schema) | Schema-checked IFC authoring: construct entities by attribute name with arity and type validation. |
| [`ifc-model`](/reference/crates/ifc-model) | <span class="status-implemented">Implemented</span> | — | The IFC entity graph: storage and structural queries, free of domain semantics and serialization. |
| [`ifc-schema`](/reference/crates/ifc-schema) | <span class="status-implemented">Implemented</span> | — | IFC schema as data: entity table, supertype chain, attribute names. |
| [`ifc-step`](/reference/crates/ifc-step) | <span class="status-implemented">Implemented</span> | [`ifc-model`](/reference/crates/ifc-model) | STEP physical file (ISO 10303-21) codec for the IFC model. |
| [`ifc-validate`](/reference/crates/ifc-validate) | <span class="status-implemented">Implemented</span> | [`ifc-model`](/reference/crates/ifc-model), [`ifc-schema`](/reference/crates/ifc-schema) | Schema conformance: WHERE rules, cardinality, GUID and reference integrity. |
| [`ifc-xml`](/reference/crates/ifc-xml) | <span class="status-implemented">Implemented</span> | [`ifc-model`](/reference/crates/ifc-model), [`ifc-schema`](/reference/crates/ifc-schema) | ifcXML (ISO 10303-28) codec for the IFC model. |

### Domain views

| Crate | Status | Depends on | Description |
| --- | --- | --- | --- |
| [`ifc-approval`](/reference/crates/ifc-approval) | <span class="status-implemented">Implemented</span> | [`ifc-model`](/reference/crates/ifc-model), [`ifc-schema`](/reference/crates/ifc-schema) | Bounded IFC4 approval resource semantics. |
| [`ifc-classification`](/reference/crates/ifc-classification) | <span class="status-implemented">Implemented</span> | [`ifc-model`](/reference/crates/ifc-model), [`ifc-schema`](/reference/crates/ifc-schema) | Classification systems, document references, libraries, external references. |
| [`ifc-constraint`](/reference/crates/ifc-constraint) | <span class="status-implemented">Implemented</span> | [`ifc-model`](/reference/crates/ifc-model), [`ifc-schema`](/reference/crates/ifc-schema) | Bounded IFC4 metric, objective, and constraint relationships. |
| [`ifc-control`](/reference/crates/ifc-control) | <span class="status-implemented">Implemented</span> | [`ifc-model`](/reference/crates/ifc-model), [`ifc-schema`](/reference/crates/ifc-schema) | Bounded IFC control semantics: permits, project orders, action requests, and performance history. |
| [`ifc-cost`](/reference/crates/ifc-cost) | <span class="status-implemented">Implemented</span> | [`ifc-model`](/reference/crates/ifc-model), [`ifc-schema`](/reference/crates/ifc-schema) | Cost semantics as a borrowed view over the IFC model. |
| [`ifc-element-type`](/reference/crates/ifc-element-type) | <span class="status-implemented">Implemented</span> | [`ifc-model`](/reference/crates/ifc-model), [`ifc-schema`](/reference/crates/ifc-schema) | Element, resource, and process type definitions: the IfcTypeObject catalogue. |
| [`ifc-material`](/reference/crates/ifc-material) | <span class="status-implemented">Implemented</span> | [`ifc-model`](/reference/crates/ifc-model), [`ifc-schema`](/reference/crates/ifc-schema) | Material definitions: layer sets, profile sets, constituents, usage. |
| [`ifc-occurrence`](/reference/crates/ifc-occurrence) | <span class="status-implemented">Implemented</span> | [`ifc-model`](/reference/crates/ifc-model), [`ifc-schema`](/reference/crates/ifc-schema) | Built element and distribution occurrence classes and their type pairing. |
| [`ifc-properties`](/reference/crates/ifc-properties) | <span class="status-implemented">Implemented</span> | [`ifc-model`](/reference/crates/ifc-model), [`ifc-schema`](/reference/crates/ifc-schema) | Property sets, quantities, and unit resolution. No geometry. |
| [`ifc-resource`](/reference/crates/ifc-resource) | <span class="status-implemented">Implemented</span> | [`ifc-model`](/reference/crates/ifc-model), [`ifc-schema`](/reference/crates/ifc-schema) | Construction resources: labour, equipment, material, crew, subcontract. |
| [`ifc-schedule`](/reference/crates/ifc-schedule) | <span class="status-implemented">Implemented</span> | [`ifc-model`](/reference/crates/ifc-model), [`ifc-schema`](/reference/crates/ifc-schema) | IFC scheduling: IfcTask/IfcWorkSchedule, sequencing, 4D linkage. |
| [`ifc-spatial`](/reference/crates/ifc-spatial) | <span class="status-implemented">Implemented</span> | [`ifc-model`](/reference/crates/ifc-model), [`ifc-schema`](/reference/crates/ifc-schema) | IFC spatial containment and objectified relationship traversal: project, site, building, storey, element. |
| [`ifc-structural`](/reference/crates/ifc-structural) | <span class="status-implemented">Implemented</span> | [`ifc-model`](/reference/crates/ifc-model), [`ifc-schema`](/reference/crates/ifc-schema) | Structural analysis model: members, connections, actions, reactions, loads. |
| [`ifc-style`](/reference/crates/ifc-style) | <span class="status-implemented">Implemented</span> | [`ifc-model`](/reference/crates/ifc-model), [`ifc-schema`](/reference/crates/ifc-schema) | Presentation: styles, colours, textures, layers, annotation. |
| [`ifc-systems`](/reference/crates/ifc-systems) | <span class="status-implemented">Implemented</span> | [`ifc-model`](/reference/crates/ifc-model), [`ifc-schema`](/reference/crates/ifc-schema) | Distribution systems, ports, and connectivity between elements. |
| [`ifc-tabular`](/reference/crates/ifc-tabular) | <span class="status-implemented">Implemented</span> | [`ifc-model`](/reference/crates/ifc-model), [`ifc-schema`](/reference/crates/ifc-schema) | Structured IFC value containers indexed by position or time. |
| [`ifc-template-catalog`](/reference/crates/ifc-template-catalog) | <span class="status-implemented">Implemented</span> | [`ifc-schema`](/reference/crates/ifc-schema) | Versioned IFC PSD/QTO catalog definitions and correction overlays |

### Geometry, georeferencing and alignment

| Crate | Status | Depends on | Description |
| --- | --- | --- | --- |
| [`ifc-alignment`](/reference/crates/ifc-alignment) | <span class="status-partial">Partial</span> | [`ifc-model`](/reference/crates/ifc-model), [`ifc-schema`](/reference/crates/ifc-schema) | IFC4x3 linear positioning: alignments, referents, linear placement, spirals. |
| [`ifc-geometry`](/reference/crates/ifc-geometry) | <span class="status-partial">Partial</span> | [`ifc-alignment`](/reference/crates/ifc-alignment), [`ifc-model`](/reference/crates/ifc-model), [`ifc-schema`](/reference/crates/ifc-schema) | IFC semantic views lowered into the format-neutral geometry DAG. |
| [`ifc-georef`](/reference/crates/ifc-georef) | <span class="status-implemented">Implemented</span> | [`ifc-model`](/reference/crates/ifc-model), [`ifc-schema`](/reference/crates/ifc-schema) | Georeferencing: map conversion, coordinate reference systems, site placement. |

### Language bindings

| Crate | Status | Depends on | Description |
| --- | --- | --- | --- |
| [`openbim-ifc-binding-core`](/reference/crates/openbim-ifc-binding-core) | <span class="status-implemented">Implemented</span> | [`openbim-ifc`](/reference/crates/openbim-ifc) | Host-independent core shared by the openbim-ifc language bindings (WebAssembly, C ABI, Python). |
| [`openbim-ifc-capi`](/reference/crates/openbim-ifc-capi) | <span class="status-implemented">Implemented</span> | [`openbim-ifc-binding-core`](/reference/crates/openbim-ifc-binding-core) | Versioned, memory-safe C ABI for openbim-ifc: read, edit and write IFC STEP files from C and C++. |
| [`openbim-ifc-dotnet`](/reference/crates/openbim-ifc-dotnet) | <span class="status-implemented">Implemented</span> | — | The OpenBim.Ifc NuGet package: .NET bindings for openbim-ifc over its versioned C ABI. |
| [`openbim-ifc-py`](/reference/crates/openbim-ifc-py) | <span class="status-implemented">Implemented</span> | [`openbim-ifc-binding-core`](/reference/crates/openbim-ifc-binding-core) | Python bindings for openbim-ifc: read, edit and write IFC STEP files from Python. |
| [`openbim-ifc-wasm`](/reference/crates/openbim-ifc-wasm) | <span class="status-implemented">Implemented</span> | [`openbim-ifc-binding-core`](/reference/crates/openbim-ifc-binding-core) | WebAssembly bindings for openbim-ifc: read, edit and write IFC STEP files from JavaScript. |

### Command-line tool

| Crate | Status | Depends on | Description |
| --- | --- | --- | --- |
| [`openbim-ifc-cli`](/reference/crates/openbim-ifc-cli) | <span class="status-partial">Partial</span> | [`openbim-ifc`](/reference/crates/openbim-ifc) | The openbim-ifc command: validate, convert and inspect IFC files (STEP and ifcXML) from a shell or CI. |

<!-- CRATES:MAP:END -->

## Not in this repository

- The generic ISO 10303-21 STEP and ISO 10303-11 EXPRESS machinery lives in
  [`openbim-step`](https://crates.io/crates/openbim-step), shared with the
  other standard families.
- Geometry evaluation (meshing, booleans, sections) lives in
  [Axiolid](https://axiolid.github.io/kernel/); see
  [the Axiolid boundary](/architecture/axiolid-boundary).
- Other openBIM standards (IDS, BCF, bSDD, …) are separate repositories
  under [openbimrs](https://github.com/openbimrs).
