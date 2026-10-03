# Changelog -- openbim-ifc-py

All notable changes to the `openbim-ifc-py` crate are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate follows Semantic Versioning independently of its siblings:
a release here does not imply a release of any other crate in the family.

## [Unreleased]

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
