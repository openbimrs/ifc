# Changelog -- ifc-classification

All notable changes to the `ifc-classification` crate are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate follows Semantic Versioning independently of its siblings:
a release here does not imply a release of any other crate in the family.

Changes up to and including 0.2.0 are recorded family-wide in the
[repository changelog](../CHANGELOG.md); that file is the archive for
everything released before per-crate changelogs began.

## [Unreleased]

## [0.2.1] - 2026-09-25

### Added

- Views read IFC2X3 models against the IFC2X3 schema (#51). A header
  declaring exactly `IFC2X3` binds the IFC2X3 table; every slot position,
  select, and domain comes from it. Before, every model was read with IFC4
  positions, so IFC2X3 files only worked where the two releases happen to
  share a slot.
  - `IfcRelAssociatesClassification.RelatingClassification` accepts IFC2X3
    `IfcClassificationNotationSelect` (notation or reference), and
    `RelatedObjects` is checked against IFC2X3 `IfcRoot`.
  - IFC2X3 `ItemReference` and `DocumentId` are read by the IFC4-named
    `identification()` accessors, since they keep position and meaning.
  - `IfcClassificationReference.ReferencedSource` follows the declared
    release: `IfcClassification` only in IFC2X3.
- `classification_schema(model)` reports the release reads bind to.
  `SchemaVersion` is re-exported.
- `ClassificationNotation`, `ClassificationView::notations()`,
  `notation()`, and `notation_values()`: an IFC2X3 notation's code is its
  facets' `NotationValue`s, in authored order, with no invented separator.
- `ClassificationError::NotInSchema`: an accessor for an attribute the
  release lacks (for example `description()` on an IFC2X3
  `IfcClassification`, or any `IfcExternalReferenceRelationship` read in an
  IFC2X3 model) fails with this instead of returning `Ok(None)`, which
  would read as "authored as empty".
- `ClassificationError::StructuredValue`: a text accessor that meets an
  attribute the release types as a record, such as an IFC2X3
  `IfcCalendarDate` `EditionDate`, returns the record id instead of
  rejecting a valid value as `InvalidValue`.
- `ClassificationError::MultipleSchemas`: a header declaring several
  schemas including IFC2X3 cannot be bound to one release.

### Unchanged

- IFC4 and undeclared (in-memory) models read exactly as in 0.2.0: every
  existing IFC4 test passes untouched. On two real IFC4 models, all 1,482
  classified objects resolve.
- IFC4X3 headers still read against IFC4, as in 0.2.0. Their own table is
  out of scope here.
- Authoring stays IFC4-only.
- Projections built with `try_new` have no model header, so they keep
  reading against IFC4.

### Verified

- On five real IFC2X3 models (two ArchiCAD, Solibri, a structural model,
  Revit), all 3,672 classified objects resolve their effective
  classifications, and every classification system (5) and reference (62)
  reads, IFC2X3 calendar edition dates included.

## [0.2.0] - 2026-09-22

First release under per-crate versioning. See the
[repository changelog](../CHANGELOG.md) for the family-wide history
that produced this version.

[Unreleased]: https://github.com/openbimrs/ifc/compare/ifc-classification-v0.2.1...HEAD
[0.2.1]: https://github.com/openbimrs/ifc/releases/tag/ifc-classification-v0.2.1
[0.2.0]: https://github.com/openbimrs/ifc/releases/tag/v0.2.0
