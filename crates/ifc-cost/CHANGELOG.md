# Changelog -- ifc-cost

All notable changes to the `ifc-cost` crate are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate follows Semantic Versioning independently of its siblings:
a release here does not imply a release of any other crate in the family.

Changes up to and including 0.2.0 are recorded family-wide in the
[repository changelog](../../CHANGELOG.md); that file is the archive for
everything released before per-crate changelogs began.

## [Unreleased]

## [0.2.2] - 2026-09-28

### Added

- `CostAuthoringError::MultipleSchemas`, `UnsupportedSchema`,
  `EntityNotInSchema` and `AuthoringNotInSchema` for release-bound quantity
  authoring (#190), and a re-export of `SchemaVersion`, which they name.
  `CostAuthoringError` is `#[non_exhaustive]`, so this is not breaking. The
  crate now depends on `ifc-schema` for the bundled release tables.

### Fixed

- `mutation::create_quantity` writes the quantity value bare (#190). It
  wrote `IFCQUANTITYAREA('Q',$,$,IFCAREAMEASURE(12.5),$)`. But
  `<Kind>Value` is declared with a defined measure type, not a SELECT, in
  IFC2X3, IFC4 and IFC4X3, and ISO 10303-21 writes a typed parameter only
  in a SELECT slot. It now writes `12.5`, and a count as the integer `4`.
  `CostQuantity::value` reads both forms.
- `mutation::create_quantity` binds the model's declared release, with the
  same rule and output as `ifc-properties`' `create_quantity` (#190). It
  wrote the IFC4 five-attribute layout into every model. Now attributes are
  placed by name from the release's table, so an IFC2X3 quantity has four.
  The signature is unchanged; the refusals are new:
  - a `formula` in IFC2X3 (`AuthoringNotInSchema`);
  - `QuantityKind::Number` outside IFC4X3 (`EntityNotInSchema`);
  - a header declaring several schemas (`MultipleSchemas`);
  - a header declaring one without a bundled table (`UnsupportedSchema`).

  A model without `FILE_SCHEMA` binds IFC4 as before. **Behaviour change:**
  such a model now refuses `QuantityKind::Number`, which IFC4 does not
  declare; declare IFC4X3 to author one. IFC4 and IFC4X3 records are
  otherwise unchanged apart from the bare value, and match `ifc-properties`
  byte for byte in all three releases.

## [0.2.1] - 2026-09-27

### Added

- `nesting_anomalies(model)` and `CostAnomaly::NestedTwice { item, kept,
  rejected, relation }` (#57). A cost item that two `IfcRelNests` place
  under different parents is reported; `Nests` is `SET [0:1]`.

### Fixed

- A cost item nested under two parents is no longer counted twice (#57).
  `parent_of` already returned the first parent, but `children_of` listed
  the item under both. So `descendants_of` and `rolled_up_total` included it
  under each parent, and summing over `roots()` double-counted its value.
  Now only the kept parent (first `IfcRelNests` in file order) lists it,
  and a child listed twice under one parent is listed once. Output changes
  only for files that violate the schema.

## [0.2.0] - 2026-09-22

First release under per-crate versioning. See the
[repository changelog](../../CHANGELOG.md) for the family-wide history
that produced this version.

[Unreleased]: https://github.com/openbimrs/ifc/compare/ifc-cost-v0.2.2...HEAD
[0.2.2]: https://github.com/openbimrs/ifc/releases/tag/ifc-cost-v0.2.2
[0.2.1]: https://github.com/openbimrs/ifc/releases/tag/ifc-cost-v0.2.1
[0.2.0]: https://github.com/openbimrs/ifc/releases/tag/v0.2.0
