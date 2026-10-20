# Changelog -- openbim-ifc-binding-core

All notable changes to the `openbim-ifc-binding-core` crate are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate follows Semantic Versioning independently of its siblings:
a release here does not imply a release of any other crate in the family.

## [Unreleased]

### Added (#474, tables)

- `IfcModel::export_table(query, columns, format, key, null)` returns the
  selected entities as CSV, TSV or XLSX bytes; `IfcModel::import_table(data,
  format, key, null, dry_run)` applies a table by key as one checked
  transaction and returns a `TableImport` record (`applied`,
  `TableChange`s, `TableRefusal`s with a code and span). Refusals of the
  table are data; nothing is written when there is any.
- Feature `sheet-xlsx` (default) for XLSX; without it XLSX refuses with
  `feature-disabled`. CSV and TSV are in every build.
- `InvalidText` maps to `type-mismatch`, `ConflictingWrite` to
  `invalid-value`.

Semver: additive (unpublished crate).

### Changed (#506, #509)

- `product_curves` polylines trimmed, composite, offset and intrinsic
  curves, which were `unsupported` refusals naming the curve. A curve the
  reference compiler has no certified polyline for (an implicit curve, an
  offset of an offset) is still `unsupported`, now naming the product,
  its message the compiler's reason. Advanced B-reps whose edges cover
  part of their curve mesh (#509). No record or code changes.

Semver: patch (internal crate, not published); fewer refusals for every
binding.

### Changed (#515, wire format 1.4)

- `GEOMETRY_FORMAT_VERSION` is `"1.4"`, the newest version a build writes
  (`axiolid-model` 0.3.14: `BoundedHalfSpaceAlongZ`, axiolid/kernel#321).
  An envelope still carries the lowest version its content needs: 1.4 only
  when a polygonal bounded half-space clips. A reader on `axiolid-model`
  0.3.13 or older refuses 1.4. Tests hold such a payload to 1.4.

Semver: additive (internal crate, not published); a minor change of every
binding (ADR 0021).

### Added (#458, curves and surfaces as output primitives)

- `IfcModel::product_curves(products, representation)` (feature `mesh`):
  a `ProductCurves` record per product (id, global id, type name,
  transform, vertex count, `ProductPolyline` records, refusal) with one
  `f32` position array; each `ProductPolyline` names its slice
  (`first_vertex`, `vertex_count`), `closed`, the IFC `curve`,
  `chord_bound` and `chord_requested`. A trimmed, composite, offset or
  intrinsic curve is an `unsupported` refusal naming the curve (#506).
- `IfcModel::product_meshes_for(products, representation)`, and
  `ProductMesh::closure` (`solid`, `surface`, `unknown`), the record's
  last field.
- Representation names `Body`, `Plan`, `Axis`, `FootPrint`, `Annotation`,
  `Surface`; another is refused with `invalid-value`.

Semver: additive (internal crate, not published).


### Added (#462, INVERSE attributes by name)

- `IfcModel::attribute_by_name` resolves an `INVERSE` name through a
  reverse index cached per model revision: a `list` of `ref`s for a `SET`
  or `BAG` (`SET [0:1]` included), a `ref` or `null` for a single-valued
  inverse, `invalid-model` when several entities reference a single one.
- `IfcModel::inverse_attribute_names(id)` (`InverseAttributeInfo`
  records), `inverse_references(id)` (`InverseReference { id, index }`)
  and `total_inverses(id)`.
- Error code `inverse-attribute`: a by-name write to an `INVERSE` name.

Semver: additive (internal crate, not published).

### Added (#481, shape representations)

- Batch operations `shape_representation` (`context`, `items`,
  `identifier`, `representation_type`), `assign_representation`
  (`representation`, `to`), `unassign_representation` (`representation`,
  `from`), `copy_representation` and `remove_representation`
  (`representation`) in `OPS`, read into the facade's operations.
- `IfcModel::remove_representation(id, dry_run)` returning the
  `RemovalReport` record.
- Refusal codes: `invalid-value` for a representation type the items do not
  fit, items no type fits, a missing identifier or a topological item;
  `invalid-model` for a representation already used, a shared product shape
  or a representation not assigned where it is unassigned from;
  `unsupported` for a type check needing an underivable curve dimension.

### Added (#464, selectors and values by path)

- `IfcModel::select(query)`, `select_from(query, base)`,
  `get_value(id, path)` (a `Selected`: a tagged value, a list or a
  property set's named members; `Selected::to_tagged` for the C tape) and
  `set_value(id, path, Plain)`, which coerces a plain value against an
  attribute's declared type or wraps it in the type of the property
  already stated. `canonical_selector`, `SELECTOR_GRAMMAR_VERSION`.
- The error code `invalid-selector` (`BindingError::InvalidSelector`, with
  `BindingError::span`) for a selector or path that does not parse or
  names a class or attribute the release lacks.
- The facade's `selector` feature is always on; property writes by path
  need `properties-write` (`feature-disabled` otherwise), so a read-only
  browser build links no edit code.

Semver: additive (internal crate, not published); a minor change of every
binding.

### Added (#480, type propagation on assignment)

- The `assign_type` operation takes `map_representations`,
  `mapping_target`, `material_usage`, `layer_set_direction` (`AXIS1` to
  `AXIS3`), `predefined_type` (`ignore`, `validate`, `sync`) and
  `replace`; `contain` and `aggregate` take `replace`. A new field kind,
  `FieldKind::Bool`, carried as a `BOOL` on the tape. An unknown mode or
  direction is `invalid-value`.
- The new refusals map to the shared codes: `RepresentationConflict` and
  `PredefinedTypeConflict` to `invalid-model`,
  `UnsupportedRepresentationMap` and `MaterialUsageUndetermined` to
  `unsupported`.

Semver: additive (a new variant of the `#[non_exhaustive]` `FieldKind`,
new optional fields), a minor release while 0.x.

### Added (#478, cascading delete and purge)

- `IfcModel::remove_product(id, RemoveProductOptions)` and
  `IfcModel::purge(PurgeOptions)`, each one checked transaction through
  the facade's `AuthorOp::RemoveProduct` and `AuthorOp::Purge`, or, with
  `dry_run`, only staged. Both return a `RemovalReport` record (`removed`
  `RemovedEntity`s, `kept` `KeptEntity`s with `reason` and `users`,
  `detached`, `rebased`, `applied`). `still-referenced` for dependent
  placements, a type with occurrences or an entity that stays and needs
  something removed; `unsupported` for a placement that cannot be
  re-based; `wrong-entity-type`, `missing-entity`, `unsupported-schema`
  (an undeclared purge class).

### Deprecated (#478)

- `IfcModel::remove`, which leaves references dangling, is deprecated in
  every host in favour of `remove_with_relationships` and `remove_product`;
  it keeps working.

Semver: additive (internal crate, not published); a minor change of every
binding.

### Added (#450)

- ifcZIP (feature `zip`, default): `parse`, `parse_with`, `parse_owned`,
  `open` and `open_mapped` recognize an archive by its ZIP signature and
  read its one STEP (under the call's options) or ifcXML model within the
  facade's default decompression budget; `write_ifczip(inner,
  xsd_profile)` writes one. Refusals map to existing codes: `parse`,
  `unsupported`, `budget-exceeded`, `invalid-value`, and
  `feature-disabled` without the feature.

### Added (#449)

- `ProductMesh` gains `colour` (`SurfaceColour`: `red`, `green`, `blue`,
  `transparency`, `source` `item`/`layer`/`material`, `source_entity`,
  `surface_style`) and `colour_refusal` (a `GeometryRefusal`, code
  `ambiguous` or a shared geometry code), the last two fields of its
  record. A product nothing colours has neither; a refused colour never
  refuses the mesh.

Semver: additive (internal crate, not published); an additive minor
change of every binding.

### Added (#463, GlobalIds, types and openings)

- `IfcModel::by_guid` through the facade's `GlobalIdIndex`, kept beside
  the model and rebuilt only when its revision or declared release
  changes; `invalid-value` for text that is not a `GlobalId`,
  `invalid-model` for one several entities carry. `duplicate_global_ids`
  (`DuplicateGlobalId { global_id, ids }`, and `duplicates_to_tagged` for
  the C tape).
- `type_object`, `occurrences_of`, `openings_of`, `fillings_of`,
  `voided_element_of`, refusing with the shared codes (`missing-entity`,
  `wrong-entity-type`, `invalid-model`, `missing-reference`).
- `new_guid()` (`unsupported` without a random source) and
  `name_based_guid(parts)`. Feature `js-random` forwards the facade's.

Semver: additive (internal crate, not published); a minor change of every
binding.

### Changed (#451)

- `IfcModel::georeferencing` reads IFC2X3 `ePSet_MapConversion` and
  `ePSet_ProjectedCRS` instead of refusing the release with
  `unsupported-schema`. Each pair crosses as a `map-conversion` record
  whose `source_kind` is the new value `property-set`, `source` the
  `IfcProject` or `IfcSite`, `operation` the `ePSet_MapConversion` set and
  `target_crs.id` the `ePSet_ProjectedCRS` set. A malformed or incomplete
  set is `invalid-model`.

Semver: additive (internal crate, not published); a minor change of every
binding (ADR 0021).

### Changed (#433, wire format 1.3)

- `GEOMETRY_FORMAT_VERSION` is `"1.3"`, the newest version a build writes
  (`axiolid-model` 0.3.12: run stations at their basis's ends,
  axiolid/kernel#303). An envelope still carries the lowest version its
  content needs: 1.3 only when an offset run continues to a basis end the
  kernel measures. A reader on `axiolid-model` 0.3.11 or older refuses
  1.3. Tests hold such a payload to 1.3 and the others to their old
  versions.

Semver: additive (internal crate, not published); a minor change of every
binding (ADR 0021).

### Changed (#429, wire format 1.2)

- `GEOMETRY_FORMAT_VERSION` is `"1.2"`, the newest version a build writes
  (`axiolid-model` 0.3.11: seam windows on the sections of a run,
  axiolid/kernel#301). An envelope still carries the lowest version its
  content needs: 1.0 when window-free, 1.1 when only stations carry a
  window, 1.2 when a run's sections do. A reader on `axiolid-model` 0.3.10
  or older refuses 1.2. Tests hold a windowed run's payload to 1.2 and
  the others to their old versions.

Semver: additive (internal crate, not published); a minor change of every
binding (ADR 0021).

### Changed (#423, wire format 1.1)

- `GEOMETRY_FORMAT_VERSION` is `"1.1"`, the newest version a build writes
  (`axiolid-model` 0.3.9: a station's seam-snapping window). An envelope
  carries the lowest wire version its content needs: 1.0 unless a station
  carries a seam-snapping window, then 1.1 (`axiolid-model` 0.3.10,
  axiolid/kernel#297). A reader on `axiolid-model` 0.3.8 or older reads a
  1.0 payload and refuses 1.1. Tests hold a window-free payload to 1.0 and
  a windowed one to 1.1.

### Added (#367, geometry Level 2, ADR 0021)

- Feature `graph` (opt-in; implies `placements`, adds the facade's
  `geometry-wire`): `IfcModel::product_geometry(ids, encoding)` ->
  `ProductGeometry` records (id, global id, type name, transform, encoding,
  payload size, refusal) with the payload, each product's Body as Axiolid's
  geometry graph in its wire format 1.0, JSON (`GeometryEncoding::Json`) or
  CBOR (`GeometryEncoding::Cbor`). A product that cannot be lowered is a
  record with a typed `GeometryRefusal` (`unsupported`, `invalid-model`,
  `missing-reference`, `budget-exceeded`); without the feature the call
  refuses with `feature-disabled`. `GEOMETRY_FORMAT` and
  `GEOMETRY_FORMAT_VERSION` name what a build writes, held to Axiolid's
  constants by a test. The payload crosses beside the record, never in it.

Semver: additive (internal crate, not published).

### Added (#358, property sets of many objects)

- `IfcModel::property_sets_many(ids)`: `property_sets` of each id, or with
  `None` of every `IfcObjectDefinition`, through one
  `ifc::properties::PropertyIndex` built for the call, so the file's
  property relationships are validated once and a pass over every object
  is linear rather than quadratic. Each `ObjectPropertySets` record
  (object, sets, refusal) holds exactly the per-object answer, a refusal
  as `PropertyRefusal` (code, message); only a refusal of the whole model
  fails the call. The index never outlives the call, so nothing can go
  stale after an edit; `property_sets` is unchanged.
- Bench `property_sets` (`benchmarks/README.md`), smoke-run by the gate.

### Added (#342, plain values coerced against the declared type)

- `Plain` (null, bool, integer, real, text, entity handle, list, or an
  `Exact` tagged value) and `IfcModel::set_attribute_by_name_plain` /
  `coerce_attribute`: a plain host value coerced against the attribute's
  declared type in the declared release (`ifc::attribute_type`). A string
  to a label (bare) or an enumeration item (any case); an integer to
  `INTEGER`, or to `REAL` when exact; a real to `REAL`; a bool to
  `BOOLEAN`/`LOGICAL`; a list element by element; a handle to a reference
  checked to exist (`missing-reference`) and fit; in a SELECT the one
  member that takes the value, as a typed parameter. The rules are in the
  `coerce` module docs.
- Codes `type-mismatch` (the value does not fit) and `ambiguous-value`
  (several SELECT members take it; the message names them).

Semver: additive (internal crate, not published); `BindingError` gains two
variants.

### Added (#328, geometry, ADR 0021)

- Feature `placements` (default): `IfcModel::product_placements(ids)`,
  per product with a shape (or per id), a `ProductPlacement` record: id,
  global id, type name, `transform` (4x4 column-major, metres),
  `representation` (`SelectedRepresentation`: id, identifier,
  representation type, context, context type, context identifier, target
  view) and `refusal`.
- Feature `mesh` (opt-in): `IfcModel::product_meshes(ids)`, a
  `ProductMesh` per product: the same identity and transform, `positions`
  (`f32`, relative to the transform) and `indices` (`u32`); its record
  carries vertex and triangle counts instead of the arrays.
- `GeometryRefusal` (code, entity, message): a product that cannot be
  placed, selected or meshed is a record, never a failed call, coded with
  the shared `unsupported`, `invalid-model`, `missing-reference` or
  `budget-exceeded`. Without its feature each call refuses with
  `feature-disabled`.

Semver: additive (internal crate, not published).

### Added (#330, schema-checked entity creation)

- Feature `author` (default): `IfcModel::author(ops)` and
  `author_seeded(ops, seed)` apply `AuthorOp`s as one checked transaction
  through the facade's `apply_authoring`, against the release the header
  declares, returning `AuthoringResult` (a `ToRecord`: per-operation
  `ids`, `created`, `removed`). Single calls `create_entity(type,
  attributes)` and `remove_with_relationships(id)` are one-operation
  batches. Without the feature each refuses with `feature-disabled`.
- Module `authoring`: `OPS` (every operation and its fields, in snake and
  camel case, with a `FieldKind`), `op_spec`, `AuthorOp::from_tagged` (the
  tape form every host converts to), `HANDLE_BASE` and `handle(index)`.
- `BindingError::MissingAttribute` (`missing-attribute`) and
  `StillReferenced` (`still-referenced`); `invalid-model` now also covers
  an authoring batch that would give an object a second containment,
  decomposition or type, or the model a second `IfcProject`.
  `BindingError` is exhaustive, so a breaking change: minor (internal,
  unreleased).

### Added (#326, attributes by name)

- `IfcModel::attribute_names(id)` (the `AttributeInfo` records, a
  `ToRecord`, in slot order), `attribute_by_name(id, name)` and
  `set_attribute_by_name(id, name, value)`, resolved against the release
  the header declares through the facade's `attribute_slots`. Names match
  case-insensitively. An entity type the release does not declare, or a
  release the build leaves out, is `unsupported-schema`.
- `BindingError::UnknownAttribute` (`unknown-attribute`) and
  `DerivedAttribute` (`derived-attribute`). `BindingError` is exhaustive,
  so a breaking change: minor (internal, unreleased).

### Added (#318, runtime PSD/QTO catalog)

- Feature `property-catalog-runtime`: the catalog is loaded at runtime
  instead of embedded. Module `catalog`: `file_name(release)`,
  `is_loaded(release)`, `load(release, bytes)` (pinned SHA-256, else
  `invalid-value`) and `RELEASES`; with the embedded catalog `is_loaded`
  is `true` and `load` a no-op, without either feature each refuses with
  `feature-disabled`.
- `BindingError::CatalogNotLoaded` (`catalog-not-loaded`): a write to a
  `Pset_`/`Qto_` set before its release's catalog is loaded, in a
  runtime-catalog build. `BindingError` is exhaustive, so a breaking
  change: minor (internal, unreleased).

### Added (#123, property sets: write side)

- `IfcModel::set_properties` (one checked transaction over a batch of
  `property_edit::PropertyEdit`s), with the one-edit forms
  `set_property` and `remove_property`; `PropertyEditResult` (a
  `Record`) names the entity holding each value afterwards.
  `PropertyEdit::from_tagged` reads the C tape form.
- Features `properties-write` (the writer, about 205 KB of a browser
  build) and `property-catalog` (the PSD/QTO catalog, 3.7 MB), both
  default. Without the first the methods refuse with `FeatureDisabled`;
  without the catalog a write to a `Pset_`/`Qto_` set does.
- `BindingError::TemplateViolation` (`template-violation`) and
  `MissingProperty` (`missing-property`). `BindingError` is exhaustive, so
  a breaking change: minor (internal, unreleased).

### Added (#123, domain views: read side)

- Read-only domain views, each an owned snapshot keyed by entity id with
  the `GlobalId` where present: `IfcModel::property_sets` (exact,
  release-bound property sets, quantity sets and predefined sets, type
  inheritance applied, values typed in the tagged encoding) and
  `resolve_unit`; `spatial_tree`; `classifications`; `material`;
  `systems`; `cost`; `georeferencing`. Records live in the new
  `properties`, `spatial`, `classification`, `material`, `systems`, `cost`
  and `georef` modules.
- The `record` module: `Record`, `Field` and `ToRecord`, the
  host-independent shape every domain record crosses in (a named, ordered
  list of fields; `Record::to_tagged` is the C tape form).
- Default features `properties`, `spatial`, `classification`, `material`,
  `systems`, `cost` and `georef` (which implies `properties`). With one
  left out, its operations refuse with `FeatureDisabled`.
- `BindingError::InvalidModel` (`invalid-model`), `MissingReference`
  (`missing-reference`), `BudgetExceeded` (`budget-exceeded`),
  `Unsupported` (`unsupported`) and `WrongEntityType`
  (`wrong-entity-type`). `BindingError` is exhaustive, so the new variants
  are a breaking change for a Rust matcher: a minor release under 0.x. The
  crate is internal and unreleased, and every host here is updated with it.

### Changed (#123)

- The `unsupported-schema` message reads `unsupported schema: <detail>`
  (was `no bundled schema for <token> in this build`), since a domain view
  refuses a bundled release it does not read with the same code. The code
  and the payload of the existing refusals are unchanged.

### Changed (#306)

- `validate`, `unreachable` and the seven domain features link only the
  schema tables of the releases the build enables; they linked every
  release's before. The default build, with all five releases, is
  unchanged. A build with one release refuses the others with
  `unsupported-schema` whichever features are on, and its test runs again
  in every such build.

### Added (#244)

- Lenient STEP reads: `ParseOptions` (`on_malformed`, `check_references`,
  `accept_real_without_point`; `strict()` and `lenient()` presets) and
  `IfcModel::parse_with`, `parse_owned_with`, `open_with` and the unsafe
  `open_mapped_with`. Recoveries are reported by `diagnostics()`.
- The STEP header: `IfcModel::header` and `set_header`, with the
  `header` module's tagged form (`to_tagged`, `from_tagged`) for the C tape.
- `IfcModel::validate(max_findings)`: validation against the declared
  schema as a `ValidationReport` of `ValidationFinding` records, sorted.
- `IfcModel::parse_ifcxml` and `write_ifcxml`: the native ifcXML layout,
  or the XSD layout of the `IFC4` / `IFC4X3_ADD2` profile.
- `IfcModel::unreachable_products`: `UnreachableProduct` records with a
  stable `reason` code.
- Default features `ifcxml`, `validate` and `unreachable`. With one left
  out, its operation refuses with `FeatureDisabled`.
- `BindingError::UnsupportedProfile` (`unsupported-profile`) and
  `BindingError::FeatureDisabled` (`feature-disabled`). `BindingError` is
  exhaustive, so for a Rust matcher the new variants are a breaking
  (minor, under 0.x) change; the crate is internal (`publish = false`) and
  unreleased, and every host in this workspace is updated with it.
- Parse and write errors name their format in the detail (`STEP: ...`,
  `ifcXML: ...`); the messages of STEP errors are unchanged.

### Added (lazy loading)

- `IfcModel::parse_owned(Vec<u8>)`, `IfcModel::open(path)` and the unsafe
  `IfcModel::open_mapped(path)`. A parsed model keeps its source and
  decodes entities on access (ADR 0015); `parse` copies the input once,
  `parse_owned` and `open` not at all beyond the file read.
- `BindingError::Io`, stable code `io`, for a file that cannot be read.

### Added

- Release features `ifc2x3`, `ifc4`, `ifc4x1`, `ifc4x2`, `ifc4x3` (all
  default), passed to `openbim-ifc`. With a release left out,
  `ids_of_type_including_subtypes` refuses a file declaring it with
  `UnsupportedSchema`, whose message says the release is not compiled in.
- The host-independent half of the language bindings (ADR 0013): `IfcModel`
  operations, the lossless `Tagged` value encoding and `BindingError` with
  stable codes, shared by the WASM, C and Python bindings. Extracted from
  `openbim-ifc-wasm`.
- Non-finite reals (NaN, infinity) are refused for every host; STEP has no
  form for them.

### Changed

- `ids_of_type_including_subtypes` resolves IFC4X1 and IFC4X2 files through
  their own bundled tables; `UnsupportedSchema` no longer lists the
  releases in its message.

