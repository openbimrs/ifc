# 0013 — Language bindings wrap the facade, one crate per target

- **Status:** Accepted
- **Date:** 2026-09-23
- **Deciders:** Friedrich Schrödter
- **Supersedes:** —

## Context

The IFC crates reach only Rust consumers (#34). JavaScript, Python and native
hosts need the same model — parse a file, read and edit entities, write it
back — without adopting Rust types or Rust's unstable ABI. The three targets
differ in calling convention and ownership, but not in what they expose.

Axiolid already ships a versioned C ABI (its ADR 0040). IFC bindings should
feel the same to a host that loads both.

## Decision

We will expose the IFC layer to other languages through **one binding crate
per target**, each depending **only on `openbim-ifc`**, never on an `ifc-*`
crate directly.

- `openbim-ifc-wasm` (browser and Node, via `wasm-bindgen`),
  `openbim-ifc-capi` (C ABI) and `openbim-ifc-py` (Python, via `pyo3`).
- *Amended 2026-09-23:* host-independent binding code -- the model
  operations, the tagged `Value` encoding and `BindingError` -- lives once,
  in `openbim-ifc-binding-core`, which depends on the facade and on no host
  toolkit. Each host crate depends on the core, not on the facade, so the
  three stay behaviourally identical: a value refused in one is refused in
  all, with the same error `code`.
- A binding adds calling-convention glue only: no IFC semantics, no
  validation, no domain logic. If a binding needs behaviour the facade lacks,
  the behaviour goes into the facade first.
- The surface is the record model — `Model`, `Entity`, `Value`, the STEP
  codec — not the domain crates. Domain projections are a later, opt-in layer.
- `Value` crosses the boundary as a **lossless tagged encoding** (a JS object
  per variant), not as native numbers and strings. IFC distinguishes `$`
  from `*`, `.U.` from `.F.`, integer from real, and typed wrappers such as
  `IFCLENGTHMEASURE(2.5)` from their payload. Folding any of these into a
  host-native value would make a round trip silently lossy.
- Each host carries that encoding in its own idiom: JS objects with
  `BigInt` integers (JS numbers lose precision above 2^53), frozen Python
  dataclasses, and a flat C node array with a string buffer (no recursive
  structs, so no Rust allocation crosses the C ABI).
- Every binding has an executable round-trip smoke test in its host language
  that parses a file, reads an entity, edits one, and writes the file back.
- *Amended 2026-10-03:* beyond the record model, the bindings carry the
  facade's non-domain surface (#244): lenient STEP reads (`ParseOptions`,
  their recoveries reported as diagnostics), the whole STEP `Header`, read
  and replaced, schema validation as structured findings, the ifcXML codec
  in its native and XSD layouts, and `unreachable_products`. Each is
  bound once in `openbim-ifc-binding-core` and carried by all three hosts
  with the same records and error codes (new: `unsupported-profile`,
  `feature-disabled`). The capabilities that add to a browser build --
  `ifcxml`, `validate`, `unreachable` -- are default features of the core
  and the WASM crate, so a size-sensitive build leaves them out as it
  leaves out IFC releases (#112); a left-out capability keeps its methods
  and refuses with `feature-disabled`, so every build has one surface.
  Checked multi-edit transactions (`Transaction`/`Applied`/`Conflict`) are
  deferred until a host asks for them. Domain views (#123) are the next
  layer, each an opt-in feature of the same kind.
- *Amended 2026-10-03 (#123, read side):* the bindings carry the facade's
  domain views as owned snapshots. A view borrows `&Model` and cannot cross
  a boundary, so `openbim-ifc-binding-core` builds each answer into plain
  records keyed by entity id, with the `GlobalId` where the entity has one,
  and every host converts them with one generic function: a record is a
  named, ordered list of fields (a JS object, a frozen Python dataclass, a
  C tape `LIST`). Bound read-only, each behind its own feature: property
  sets and quantities with type inheritance (exact, release-bound) and unit
  resolution, the spatial tree, classification, materials, systems, cost,
  and georeferencing (a new facade join of `georef` and `properties`, since
  the project length unit scales every operation). IFC values in them keep
  the tagged encoding, typed with their declared type; resolved parameters
  (thicknesses, map offsets) are host numbers. A release a view does not
  read is refused with `unsupported-schema`; new codes `invalid-model`,
  `missing-reference`, `budget-exceeded`, `unsupported` and
  `wrong-entity-type` keep the domain crates' refusals distinct. All seven
  are default features of the core and the npm package, which carries
  every capability; writing property sets is the second half of #123.
- *Amended 2026-10-03 (#123, write side):* the bindings write property
  sets as one checked edit, the first write the domain layer carries. The
  facade grows it first (`ifc::apply_property_edits`): a batch of edits,
  each addressed by object, set name and property name as the read side
  reports them, is planned against the model, staged on one
  `Transaction` and committed, or refused with nothing written. The value
  is the read side's tagged `value`. A value an occurrence inherits is
  overridden on the occurrence, never changed on the type's shared set; a
  set or property entity shared with other objects is copied before it
  changes. Values are checked against the declared release and, for a
  `Pset_`/`Qto_` set, the release's PSD/QTO catalog (ADR 0017); new sets
  take the object's owner history and a name-based `GlobalId`, since no
  randomness builds for every target. Hosts call it `setProperties` /
  `set_properties` / `openbim_ifc_v0_1_model_set_properties` with one-edit
  forms beside it; the C batch is a value tape the core reads, so the
  three cannot read an edit differently. New codes `template-violation`
  (C 25) and `missing-property` (26). The writer (`properties-write`) and
  the catalog (`property-catalog`, 3.7 MB of a browser build) are
  features of their own, default in every host; without the catalog a
  write to a `Pset_`/`Qto_` set refuses with `feature-disabled`.
- *Amended 2026-10-03 (#318, runtime catalog):* the npm package no longer
  embeds the PSD/QTO catalog. Its module is built with
  `property-catalog-runtime`; the package ships one pinned snapshot file
  per edition (`catalog/<edition>.bin`), and `await
  IfcModel.loadCatalog(release?)` reads the one a release needs (Node
  from the package directory, a browser or bundle relative to the module,
  or bytes or a base URL the caller passes), checks it against its
  SHA-256 and keeps it for the module instance. Until its release is
  loaded, a write to a `Pset_`/`Qto_` set refuses with the new code
  `catalog-not-loaded` (C 27), never unchecked; removals and other sets
  need no catalog. Once loaded, the writer checks exactly as with the
  embedded catalog. The C and Python bindings keep `property-catalog`
  (embedded), so their behaviour and codes are unchanged; the core's
  `catalog` module is the same surface in both builds.
- *Amended 2026-10-03 (#326, attributes by name):* the bindings read and
  write an entity's attributes by name as well as by position. The facade
  grows it first (`ifc::attribute_slots`, `attribute_by_name`,
  `set_attribute_by_name`): a name resolves to its slot in the release the
  header declares, never another, so `IfcTask.Status` is slot 6 of an
  IFC2X3 file and slot 7 of an IFC4 one. The slots are the explicit
  attributes in Part 21 order, inherited first; `INVERSE` attributes hold
  none. Names match ASCII case-insensitively, as EXPRESS identifiers do,
  and come back in the schema's spelling. An inherited attribute a
  subtype redeclares as derived is listed `derived`, reads as stored
  (`*`) and refuses a write. Hosts call it `attributeNames` /
  `attributeByName` / `setAttributeByName`, `attribute_names` /
  `attribute_by_name` / `set_attribute_by_name`, and
  `openbim_ifc_v0_1_entity_attribute_names` / `_entity_attribute_by_name`
  / `_entity_set_attribute_by_name`, beside the positional calls, which
  stay raw slot access. An entity type the declared release lacks is
  `unsupported-schema`; new codes `unknown-attribute` (C 28) and
  `derived-attribute` (29). A Pythonic object layer (`wall.Name`, #332)
  builds on these calls.
- *Amended 2026-10-04 (#327, .NET):* .NET hosts (Revit, Navisworks,
  Tekla, Dynamo) get the `OpenBim.Ifc` NuGet package: C# P/Invoke over
  the versioned C ABI (`openbim_ifc_v0_1_*`) and nothing else, the one
  binding that reaches the core through another binding's boundary. The
  row of the alternatives table against thin layers over the C ABI was
  about hosts with a safe Rust binding generator; .NET has none that
  covers .NET Framework, and the C ABI already holds the conventions a
  host needs, so a second Rust host crate would add a toolchain and no
  semantics. The C# adds calling-convention glue only; a capability the
  C ABI lacks goes into the C ABI first, for every host. The package
  targets `netstandard2.0` (.NET Framework 4.6.2 and later) and `net8.0`
  and carries the C ABI's shared library per runtime identifier
  (`win-x64`, `win-arm64`, `linux-x64`, `linux-arm64`, `osx-x64`,
  `osx-arm64`), taken from the C ABI's release archives rather than built
  twice; build targets copy the Windows libraries into a .NET Framework
  project, and the assembly loads them from next to itself. The model is
  an `IDisposable` over a `SafeHandle`; `Value` is a closed record
  hierarchy with value equality; `IfcException` carries the shared `Code`
  and the C status; domain records are positional C# records decoded from
  the tape by one generic function, as the core intends; attributes by
  name (#326) are `AttributeNames`, `AttributeByName` and
  `SetAttributeByName`. Two names differ
  from the other hosts: the core's `System` record is `IfcSystem` and
  `Systems` is `SystemsView`, since C# forbids or penalises the originals.
  The package lives in a thin crate, `openbim-ifc-dotnet`, with no Rust
  dependency (an architecture test pins that): it gives the package a
  version, a changelog, a release tag and a reference page through the
  existing tooling, and its tests hold the C# declarations to the C header
  and the C# records to the core's, field by field, in the gate without a
  .NET SDK. The package's own suite runs against the packed `.nupkg` from
  a fresh project, on every runtime before a release publishes it, through
  NuGet trusted publishing.
- *Amended 2026-10-04 (#332, Pythonic access):* the Python package adds
  a pure-Python layer over its calls, in the package's Python sources and
  with no native surface: `Entity` views (`model[id]`, `by_type`,
  `wall.Name` through `attribute_by_name`/`set_attribute_by_name`),
  property sets as read-only mappings over `property_sets`, and an
  optional pandas export. Its reads fold values into plain Python ones,
  the mapping the alternatives table rejects as the boundary encoding; it
  stays a convenience above that boundary, documented value by value as
  lossy where it is (`*` and `.U.` read `None`, enums `str`, typed
  wrappers their payload), and the exact tagged value is one call away
  (`raw`). Writes still take tagged values: a bare `str`, `int`, `float`
  or `bool` is refused rather than guessed. Subtype tests and type
  filters use `ids_of_type_including_subtypes`, so the layer holds no
  schema knowledge of its own.
- *Amended 2026-10-04 (#330, entity creation):* the bindings create
  entities, the first checked multi-edit transaction over arbitrary
  entities they carry. The facade grows it first (`ifc::apply_authoring`,
  feature `authoring`): a batch of operations runs in order against the
  model as the ones before it leave it and is committed as one
  `Transaction`, or refused with nothing written. Every record is built
  by attribute name through `ifc-author`'s `EntityBuilder` against the
  release the header declares, then its references are resolved to an
  accepted type and its aggregates held to their declared bounds. The
  builders are facade compositions over that one checked create (project,
  spatial element aggregated under its parent, product placed, contained
  and typed, type object and assignment, containment, aggregation, local
  placement, an owner history through `ifc-author`'s writers), and
  removal takes an entity out of every relationship; `ifc-spatial`'s
  writers lay out IFC4 unless given an owner history, so they were not
  the base. An operation names an earlier one's entity by a handle, an id
  in a reserved range (2^62 plus its position), so references cross every
  host as plain ids and `REF` values with no new value kind. Each host
  converts its operation objects field by field, guided by the core's
  `OPS` table, into one tape form the core reads (the C batch element),
  as with the property edit's tape. A new `IfcRoot` gets a name-based
  `GlobalId` over a per-batch seed: the operating system's on native
  targets, `Math.random` in the browser, where the module has none;
  `OwnerHistory` is never invented. Hosts call it `author` / `Author` /
  `openbim_ifc_v0_1_model_author`, with `createEntity` and
  `removeWithRelationships` (in each host's spelling) as one-operation
  forms. New codes `missing-attribute` (C 30) and `still-referenced`
  (31); a second containment, decomposition, typing or `IfcProject` is
  `invalid-model`.

## Alternatives considered

| Option | Why not |
| --- | --- |
| One crate exporting all three targets behind features | Each target pulls its own toolchain (`wasm-bindgen`, `pyo3`, `cbindgen`) and its own `crate-type`; mixing them makes every build pay for all three and every change risk all three. |
| Bind the `ifc-*` crates directly | Duplicates the facade's feature selection per target and lets a binding bypass the layering rules the facade enforces. |
| Duplicate the model operations per host crate | Three copies of the validation drift apart; a value one host refuses, another would accept. |
| Python and WASM as thin layers over the C ABI | Adds a second unsafe boundary and a manual memory protocol to hosts that already have safe, idiomatic Rust binding generators. |
| Map `Value` to plain host values (number, string, null) | Lossy: `$`/`*`, `.U.`, int/real and typed wrappers do not survive. |

## Consequences

**Positive**

- A consumer of any binding sees the same model the Rust facade does.
- Binding crates can version and release independently of the IFC crates.
- The layering rules keep holding: bindings are a new outermost layer.

**Negative / costs**

- The tagged `Value` encoding is more verbose than host-native values.
- WASM builds need `wasm32-unknown-unknown`-compatible dependencies; the
  gate checks this so a native-only dependency cannot slip in.

**Follow-ups / risks to watch**

- The C ABI is the one `unsafe` crate; every dereference sits in its
  `buffer` module with a `SAFETY` note, as in Axiolid's ADR 0040.
- Publication (npm, PyPI, a CMake package) is tracked separately under #34.
- npm publication needs a package name and registry credentials; until then
  the WASM package is built and tested but not published.

## Relation to existing code

- `openbim-ifc-binding-core/` (shared, host-independent)
- `openbim-ifc-wasm/`, `openbim-ifc-capi/`, `openbim-ifc-py/`
- `openbim-ifc/` (the only allowed dependency)
- `ifc-model/tests/package_architecture.rs` (enforces the dependency rule)
