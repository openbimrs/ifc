# Changelog -- openbim-ifc-py

All notable changes to the `openbim-ifc-py` crate are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate follows Semantic Versioning independently of its siblings:
a release here does not imply a release of any other crate in the family.

## [Unreleased]

### Added (#465)

- `IfcModel.set_history_limit`, `history`, `undo`, `redo` and
  `clear_history`, with the records `HistoryState` and `HistoryChange`.
  History is off until a limit is set; then every mutating method is one
  undo entry. Errors `history-empty` and `history-stale`.

Semver: additive: minor.

### Changed (#559)

- `set_properties`, `set_property`, `remove_property`, `set_value`, `import_table`: draw a seed per call for the `GlobalId`s of the sets and
  relationships they create; one the model already holds is refused with
  `invalid-value`.

Semver: patch (signatures unchanged).

### Added (#538, EXPRESS rules)

- `IfcModel.validate_express(express, max_findings=None)` validates with the
  EXPRESS rules of the declared release from the `.exp` text the caller
  supplies. `IfcError.code` gains `express-mismatch` and `invalid-express`.
- `ValidationReport.indeterminate` (default 0): EXPRESS rules that evaluated
  to UNKNOWN; findings gain the severity `indeterminate`.

Semver: additive: minor.

## [0.13.0] - 2026-10-20

### Added (#479, copy and append)

- `IfcModel.copy_product(id, *, offset, placement, openings, fillings,
  container, aggregate, dry_run)` and `IfcModel.append(source, ids, *,
  global_ids, reuse, dry_run)`, returning a `CopyReport` (with
  `CopiedEntity` records); `AuthorOp.copy_product` and
  `AuthorOp.copy_entity` for batches.

Semver: additive: minor.

### Added (#469, unit conversion)

- `openbim_ifc.convert_unit(value, from_unit, to_unit)` between units named
  as text (SI with prefix, IFC4 Table 673 names with exact factors), and
  `openbim_ifc.format_length(value, unit, *, style, places, display_unit,
  suppress_zero_inches)` (decimal, feet-inches, inches with fractions).
- `model.convert_unit_value(value, from_unit, to_unit)` between two unit
  entities of the model.
- `model.convert_length_unit(target, *, area_and_volume=True,
  remove_replaced_units=True, dry_run=False)`: rescales every length, area
  and volume value to a new project length unit as one checked transaction,
  returning a `LengthUnitReport` (`ConvertedValues`, `KeptValues`).
  Refusals use the existing codes (`type-mismatch`, `unsupported`,
  `invalid-value`, `invalid-model`).

### Added (#474, tables)

- `model.export_table(query, columns, *, format="csv", key=None,
  null="")` returns the selected entities as CSV, TSV or XLSX bytes, one
  column per value path; `model.import_table(data, *, format="csv",
  key=None, null="", dry_run=False)` applies a table back by key as one
  checked transaction and returns a `TableImport` (`applied`, `changes` of
  `TableChange`, `refusals` of `TableRefusal`). Refusals are data, not
  exceptions.

Semver: additive, a minor release while 0.x.

### Changed (#512)

- `product_curves` gives the curves of a representation that also holds text or a
  fill area: a dimension `IfcAnnotation` (its lines as `IfcPolyline`s,
  its value as an `IfcTextLiteralWithExtent`) was an `unsupported`
  refusal and now returns its lines. `ProductCurves.skipped` (a tuple of ids), the record's new last
  field, lists each `IfcTextLiteral`, `IfcTextLiteralWithExtent` and
  `IfcAnnotationFillArea` of the representation by entity id: not drawn,
  not a refusal.

Semver: minor.

## [0.12.1] - 2026-10-18

### Changed (#506, #509)

- `product_curves` returns polylines for trimmed, composite, offset and intrinsic
  curves (an `IfcTrimmedCurve` axis, an `IfcCompositeCurve` footprint, an
  IFC4X3 clothoid), which were `unsupported` refusals. A curve the
  reference compiler has no certified polyline for (an implicit curve, an
  offset of an offset) is still `unsupported`, its refusal now naming the
  product rather than the curve. Advanced B-reps whose edges cover part of
  their line, circle or ellipse mesh (#509). No API change.

Semver: patch.

## [0.12.0] - 2026-10-16

### Changed (#515, wire format 1.4)

- The geometry graph payload (#367) may be Axiolid's wire format 1.4
  (`axiolid-model` 0.3.14), a minor version of the format (ADR 0021,
  amended): every `IfcPolygonalBoundedHalfSpace` now lowers to
  `BoundedHalfSpaceAlongZ`, the half space bounded along its `Position`'s
  Z (axiolid/kernel#321), so an oblique clip such as a gable wall's under
  a sloped roof is carried as authored. An envelope carries the lowest
  version its content needs: a payload with such a clip is 1.4 (it was
  1.0), every other is unchanged. A reader on `axiolid-model` 0.3.13 or
  older refuses 1.4.
  `GEOMETRY_FORMAT_VERSION` is `"1.4"`, the newest version the wheel
  writes.

Semver: minor, as ADR 0021 states for a minor version of the wire format.

## [0.11.0] - 2026-10-14

### Added (#458, curves and surfaces as output primitives)

Semver: minor (additive).

- `model.product_curves(ids=None, representation="Plan")` (mesh wheel):
  a `ProductCurves` per product, its positions one `array('f')` relative
  to its transform and a `ProductPolyline` per curve (`first_vertex`,
  `vertex_count`, `closed`, `curve`, `chord_bound`, `chord_requested`).
  `representation` is `Plan`, `Axis`, `FootPrint`, `Annotation`, `Body`
  or `Surface`; another raises `invalid-value`. A trimmed, composite,
  offset or intrinsic curve is an `unsupported` refusal (#506).
- `model.product_meshes(ids=None, representation="Body")` meshes another
  representation on request, such as `"Surface"`, and every
  `ProductMesh` has `closure`: `"solid"`, `"surface"` or `"unknown"`.


## [0.10.0] - 2026-10-14

### Added (#462, INVERSE attributes by name)

- `entity.IsDefinedBy` and every other `INVERSE` name, resolved against
  the file's release: a tuple of `Entity` views for a `SET` or `BAG`
  (`SET [0:1]` included), an `Entity` or `None` for a single-valued
  inverse; also through `model.attribute_by_name` (`List` of `Ref`, or
  `Ref`/`Null`). `dir(entity)` lists them.
- `model.inverse_attribute_names(id)` (`InverseAttributeInfo`),
  `model.inverse_references(id)` (`InverseReference(id, index)`),
  `model.total_inverses(id)`, and `entity.inverse_attribute_names()`,
  `entity.inverse_references()` (`(Entity, index)` pairs),
  `entity.total_inverses()`.
- A write to an `INVERSE` name raises `IfcError` with the new code
  `inverse-attribute`. Reading one no longer raises `unknown-attribute`.

### Added (#481, shape representations)

- `AuthorOp.shape_representation(context, items, *, identifier=None,
  representation_type=None)`, `AuthorOp.assign_representation(representation,
  to)`, `AuthorOp.unassign_representation(representation, from_)`,
  `AuthorOp.copy_representation(representation)` and
  `AuthorOp.remove_representation(representation)` for `model.author`.
- `model.remove_representation(id, *, dry_run=False)`, returning a
  `RemovalReport`.

### Added (#464, selectors and values by path)

- `model.select(query, base=None)` (`Entity` views) and
  `model.select_ids(...)`, `model.get_value(entity, path)` (plain values,
  a property set as a read-only mapping) and
  `model.set_value(entity, path, value)`;
  `openbim_ifc.canonical_selector(query)`,
  `openbim_ifc.SELECTOR_GRAMMAR_VERSION`, `SelectedValue`.
- The code `invalid-selector`, with `IfcError.span`, the byte range an
  invalid selector refuses.

Semver: additive, a minor release while 0.x.

### Added (#480, type propagation on assignment)

- `AuthorOp.assign_type(type_object, objects, *, owner_history=None,
  map_representations=False, mapping_target=None, material_usage=False,
  layer_set_direction=None, predefined_type=None, replace=False)`, and
  `replace=False` on `AuthorOp.contain` and `AuthorOp.aggregate`. Every
  new argument is keyword-only and off by default.

Semver: additive, a minor release while 0.x.

## [0.9.0] - 2026-10-13

### Added (#478, cascading delete and purge)

- `model.remove_product(id, *, rebase_placements=False,
  untype_occurrences=False, dry_run=False)` and `model.purge(classes=(),
  *, include_roots=False, include_owner_history=False, dry_run=False)`,
  returning a `RemovalReport` (`RemovedEntity`, `KeptEntity`).

### Deprecated (#478)

- `model.remove(id)` leaves references dangling and now warns with a
  `DeprecationWarning`; use `remove_with_relationships` or
  `remove_product`. It keeps working.

Semver: additive, a minor release while 0.x.

### Added (#450)

- `IfcModel.write_ifczip(*, inner="step", xsd_profile=None)`, and ifcZIP
  input to `IfcModel.parse` and `IfcModel.open`, recognized by the ZIP
  signature and read within a decompression budget (`parse`,
  `unsupported` or `budget-exceeded` otherwise).

Semver: minor (additive).

### Added (#449)

- `ProductMesh.colour` (`SurfaceColour`: `red`, `green`, `blue`,
  `transparency`, `source` `item`/`layer`/`material`, `source_entity`,
  `surface_style`) and `ProductMesh.colour_refusal` (a `GeometryRefusal`,
  code `ambiguous` or a geometry code), both defaulting to `None`, in a
  wheel built with `mesh`: the colour of the Body's item style, else its
  presentation layer's, else its material's styled representation.
  `SurfaceColour` is exported from `openbim_ifc`.

### Added (#463, GlobalIds, types and openings)

- `model.by_guid(global_id)` (an `Entity` or `None`) and
  `model.id_by_guid(global_id)`, through an index built on the first lookup;
  a `GlobalId` several entities carry raises `IfcError` (`invalid-model`),
  and `model.duplicate_global_ids()` lists them as `DuplicateGlobalId`.
- `model.type_object(id)`, `occurrences_of`, `openings_of`, `fillings_of`
  and `voided_element_of`, in IFC2X3, IFC4 and IFC4X3.
- `openbim_ifc.new_guid()` and `openbim_ifc.name_based_guid(*parts)`.

Semver: additive, a minor release while 0.x.

### Changed

- `IfcModel.georeferencing()` reads IFC2X3 files through their `ePSet_MapConversion` and
  `ePSet_ProjectedCRS` property sets instead of refusing them as
  `unsupported-schema` (#451); such a record's source kind is
  `property-set`. Through `openbim-ifc-binding-core`.

Semver: minor (ADR 0021).

## [0.8.0] - 2026-10-13

### Changed (#433, wire format 1.3)

- The geometry graph payload (#367) may be Axiolid's wire format 1.3
  (`axiolid-model` 0.3.12), a minor version of the format (ADR 0021,
  amended): an `IfcOffsetCurveByDistances` along an offset whose length is
  a quadrature now lowers, continued to the basis's ends the kernel
  measures. An envelope carries the lowest version its content needs, so
  every other payload is unchanged: 1.0, 1.1 or 1.2. A reader on
  `axiolid-model` 0.3.11 or older refuses 1.3.
  `GEOMETRY_FORMAT_VERSION` is `"1.3"`, the newest version the wheel
  writes.

Semver: minor, as ADR 0021 states for a minor version of the wire format.

## [0.7.0] - 2026-10-12

### Changed (#429, wire format 1.2)

- The geometry graph payload (#367) may be Axiolid's wire format 1.2
  (`axiolid-model` 0.3.11), a minor version of the format (ADR 0021,
  amended): a sectioned solid or surface along an offset whose length is a
  quadrature now lowers, its sections carrying seam-snapping windows.
  An envelope carries the lowest version its content needs, so every other
  payload is unchanged: 1.0, or 1.1 when a station carries a window. A
  reader on `axiolid-model` 0.3.10 or older refuses 1.2. `GEOMETRY_FORMAT_VERSION` is `"1.2"`, the newest version the wheel writes.

Semver: minor, as ADR 0021 states for a minor version of the wire format.

## [0.6.1] - 2026-10-12

### Fixed (axiolid/kernel#297)

- Window-free payloads are labelled 1.0 again, readable by `axiolid-model`
  0.3.8: built with `axiolid-model` 0.3.10, the geometry graph payload's
  envelope carries the lowest wire format version its content needs, 1.0
  unless a station carries a seam-snapping window, then 1.1. `GEOMETRY_FORMAT_VERSION` stays `"1.1"`, the newest version the wheel writes.

Semver: patch.

## [0.6.0] - 2026-10-10

### Changed (#423, wire format 1.1)

- The geometry graph payload (#367) is Axiolid's wire format 1.1
  (`axiolid-model` 0.3.9), a minor version of the format (ADR 0021,
  amended): a station on an offset whose length is a quadrature now lowers
  with a seam-snapping window. An envelope carries the lowest wire version
  its content needs: 1.0, or 1.1 when a station carries a seam-snapping
  window; `axiolid-model` 0.3.9 labels every payload 1.1
  (axiolid/kernel#297), and a reader on `axiolid-model` 0.3.8 or older
  refuses 1.1. `GEOMETRY_FORMAT_VERSION` is `"1.1"`, the newest version
  the wheel writes.

Semver: minor, as ADR 0021 states for a minor version of the wire format.

## [0.5.0] - 2026-10-10

### Added (#367, geometry Level 2, ADR 0021)

- `IfcModel.product_geometry(ids=None, encoding="json")` ->
  `ProductGeometry` (transform, encoding, payload size, refusal, payload),
  each product's Body as Axiolid's geometry graph in its wire format 1.0,
  a `str` (JSON) or `bytes` (`encoding="cbor"`), lowered with the GIL
  released; `GEOMETRY_FORMAT` and `GEOMETRY_FORMAT_VERSION`. Cargo feature
  `graph`, opt-in: the published wheel raises `feature-disabled`; build
  with `maturin build --release --features graph`.

Semver: additive, a minor release while 0.x.

## [0.4.0] - 2026-10-08

### Added (#358, property sets of many objects)

- `IfcModel.property_sets_many(ids=None)`: one `ObjectPropertySets`
  (object, sets, `PropertyRefusal`) per id, or per object definition, in
  one pass (the GIL released), each exactly what `property_sets` answers.
  `to_dataframe` resolves its rows through it, so it is linear in the
  model.

### Changed (#342, plain values coerced against the declared type)

- `wall.Name = "x"` works: `Entity` writes and the new
  `IfcModel.set_attribute_by_name_plain` coerce a plain `str`, `int`,
  `float`, `bool`, list or tuple against the attribute's declared type
  (a label written bare, an enumeration item in any case, a typed SELECT
  member when exactly one takes it). Bare values no longer raise
  `TypeError`; a value that does not fit raises `type-mismatch`, one
  several SELECT members take `ambiguous-value`. Tagged values are still
  written exactly.
- An `Entity` written as a reference is now checked to be of a type the
  attribute accepts (`type-mismatch`); write `Ref(id)` to bypass the
  check.

Semver: a behaviour change for writes that were refused (bare values) or
unchecked (an `Entity` of the wrong type): a minor release under 0.x.

## [0.3.3] - 2026-10-08

### Added (#328, geometry, ADR 0021)

- `IfcModel.product_placements(ids=None)`: `ProductPlacement` dataclasses
  with `transform` (16 floats, column-major, metres), the selected Body
  `representation` (`SelectedRepresentation`) and a typed `refusal`
  (`GeometryRefusal`) per product.
- `IfcModel.product_meshes(ids=None)`, compiled with the GIL released:
  `ProductMesh` dataclasses with `positions` (`array('f')`, relative to
  `transform`) and `indices` (`array('I')`). Cargo feature `mesh`,
  opt-in: the published wheel raises `feature-disabled`; build with
  `maturin build --release --features mesh`.

Semver: additive, a patch release.

## [0.3.2] - 2026-10-04

### Added (#332, Pythonic access)

- A pure-Python layer over the existing calls, with no new native
  surface: `openbim_ifc.open(path)`; `model[id]`, `model.by_id(id)`,
  `model.by_type(name, include_subtypes=True)`, `iter(model)` and
  `id in model`, returning `Entity` views (new module
  `openbim_ifc.entity`). An `Entity` reads attributes by name as plain
  Python values (`wall.Name`, `wall.get(name)`), keeps the exact tagged
  value reachable (`wall.raw(name)`), writes through
  `set_attribute_by_name` (`wall.Name = Text("x")`, `wall.set(...)`;
  bare `str`/`int`/`float`/`bool` refused), compares and hashes by id,
  and offers `is_a(type)` with subtypes, `psets` and `qtos` as read-only
  mappings over `property_sets`, and `set_property`/`remove_property`.
  The plain conversion is lossy for `*` and `.U.` (both `None`), enums
  (`str`) and typed wrappers (payload only); the Python guide tabulates it.
- `IfcModel.to_dataframe(type="IfcProduct", psets=True, qtos=True, ...)`
  behind the new optional extra `openbim-ifc[pandas]`, imported lazily.
- Types: `Entity`, `PlainValue`, `PropertyValue` and `Assignable` are
  exported; the test run checks a typed example and the guide's snippets
  with `mypy --strict`.
- Additive: a patch release. `iter(model)` and `in`, which raised
  `TypeError` before, now work; no existing call changes.

### Added (#330, schema-checked entity creation)

- `IfcModel.author(ops)`: `AuthorOp`s built with `AuthorOp.create`,
  `.edit`, `.remove`, `.project`, `.spatial`, `.product`, `.type_object`,
  `.assign_type`, `.contain`, `.aggregate`, `.placement` and
  `.owner_history`, applied as one checked transaction and returning
  `AuthoringResult`; `openbim_ifc.handle(index)` (and `HANDLE_BASE`) names
  the entity an earlier operation produced. `create_entity(type_name,
  attributes)` and `remove_with_relationships(id)` are one-operation
  batches. New codes `missing-attribute` and `still-referenced`. Additive:
  a patch release.

## [0.3.1] - 2026-10-04

### Added (#326, attributes by name)

- `IfcModel.attribute_names(id)` (frozen `AttributeInfo` records in slot
  order), `attribute_by_name(id, name)` and `set_attribute_by_name(id,
  name, value)`, resolved against the release the header declares; names
  match case-insensitively. Error codes `unknown-attribute` and
  `derived-attribute`. Additive: a patch release.

## [0.3.0] - 2026-10-03

### Added (#123, property sets: write side)

- `IfcModel.set_properties(edits)` over frozen `PropertyEdit`s
  (`PropertyEdit.removal(...)` removes), returning a
  `PropertyEditResult`; `set_property(object, set, name, value,
  set_type=None)` and `remove_property(object, set, name)`. Error codes
  `template-violation` and `missing-property`. The wheel carries the
  PSD/QTO catalog.
- Additive: a patch release.

### Added (#123, domain views: read side)

- `IfcModel.property_sets(id)`, `resolve_unit(measure_type, unit=None)`,
  `spatial_tree()`, `classifications(id)`, `material(id)`, `systems()`,
  `cost()` and `georeferencing()`, returning frozen dataclasses from the
  new `openbim_ifc.domains` module (`PropertySet`, `Property`,
  `SpatialTree`, `Classification`, `MaterialAssignment`, `Systems`,
  `Cost`, `MapConversion`, ...), all exported from `openbim_ifc`.
- Error codes `invalid-model`, `missing-reference`, `budget-exceeded`,
  `unsupported` and `wrong-entity-type`.
- Additive: a patch release.

## [0.2.1] - 2026-10-03

### Added (#244)

- `IfcModel.parse(data, options=...)` and `IfcModel.open(path, options=...)`
  take a frozen `ParseOptions` (`ParseOptions.lenient()` skips damaged
  records); recoveries are listed by `diagnostics()`.
- `IfcModel.header` (a frozen `Header`) and `set_header()`.
- `IfcModel.validate(max_findings=None)`: a frozen `ValidationReport` of
  `ValidationFinding`s.
- `IfcModel.parse_ifcxml()` and `write_ifcxml()`, with an optional
  `xsd_profile` (`"IFC4"`, `"IFC4X3_ADD2"`).
- `IfcModel.unreachable_products()`: `UnreachableProduct` records.
- Error codes `unsupported-profile` and `feature-disabled`.
- Additive: a patch release.

## [0.2.0] - 2026-09-29

### Changed

- Links every bundled IFC release explicitly through the binding core's
  new release features; behaviour is unchanged.

### Added (lazy loading)

- `IfcModel.open(path, *, mapped=False)`: read a file straight into the
  model with the GIL released; `mapped=True` memory-maps it (the file must
  not change while the model lives). Failures raise `IfcError` with the
  new code `io`.
- Parsed models decode entities on first access (ADR 0015), so opening a
  large file is several times faster and holds the file plus what was
  touched. `IfcModel.parse` now copies its input once, not twice.

## [0.1.0] - 2026-09-26

### Added

- Opt-in `rusty_alloc` feature (off by default): the extension's Rust
  allocations go through the pure-Rust rusty_alloc allocator, pinned to
  exactly 2.2.1. Reading STEP into a model takes 18-35% less CPU time on
  seven real IFC files, at 1-7% less peak memory; the models are identical
  on 2,273 corpus files. Build with
  `maturin build --release --features rusty_alloc`. It replaces the C
  `mimalloc` feature, which cost more CPU time than the system allocator on
  a host with transparent huge pages set to `always` (#49).

- Python bindings for the IFC facade (#39, ADR 0013), built with pyo3 and
  maturin as one abi3 wheel for CPython 3.9+. `IfcModel` parses and writes
  IFC STEP, queries by exact type or including subtypes, reads and edits
  attributes, adds and removes entities, and reports dangling references.
- Attribute values are frozen dataclasses (`Null`, `Derived`, `Unknown`,
  `Typed`, ...); bare Python values are refused rather than guessed.
- Every failure raises `IfcError` with the stable `code` shared with the
  C and JavaScript bindings. Parsing releases the GIL.
