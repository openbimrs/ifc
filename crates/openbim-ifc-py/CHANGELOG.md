# Changelog -- openbim-ifc-py

All notable changes to the `openbim-ifc-py` crate are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate follows Semantic Versioning independently of its siblings:
a release here does not imply a release of any other crate in the family.

## [Unreleased]

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
