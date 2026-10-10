# Changelog -- openbim-ifc-py

All notable changes to the `openbim-ifc-py` crate are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate follows Semantic Versioning independently of its siblings:
a release here does not imply a release of any other crate in the family.

## [Unreleased]

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
