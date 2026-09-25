# Changelog -- ifc-systems

All notable changes to the `ifc-systems` crate are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate follows Semantic Versioning independently of its siblings:
a release here does not imply a release of any other crate in the family.

Changes up to and including 0.2.0 are recorded family-wide in the
[repository changelog](../CHANGELOG.md); that file is the archive for
everything released before per-crate changelogs began.

## [Unreleased]

## [0.2.1] - 2026-09-25

### Fixed

- IFC2X3 models are read against the IFC2X3 schema (#52). Before, every
  model was read with IFC4 ancestry, which gave wrong answers on IFC2X3:
  - `systems()` listed `IfcZone` as a system and rejected
    `IfcElectricalCircuit` as `NotASystem`. In IFC2X3 a zone is a plain
    group and an electrical circuit is a system; both are now read that way.
    A relationship assigning members to an IFC2X3 zone is still reported by
    `systems()` as `NotASystem`, since a zone is not a system there.
  - `zones()` checks WR1 against the declared release. IFC2X3 admits
    `IfcZone` and `IfcSpace` members only; it has no `IfcSpatialZone`.
  - `ElementRole::of`, `ports()`, and `ConnectionGraph::build` use the
    declared release's ancestry, so an IFC4-only record in an IFC2X3 file
    (such as `IfcPipeSegment`) has no role.

### Added

- `schema_of(model)` reports the release reads bind to, or why none could
  be bound (`SchemaResolutionError`: missing, multiple, or unsupported,
  including IFC4X3). `SchemaVersion` is re-exported.
- `long_name_of(model, zone)` reads `IfcZone.LongName` and returns
  `SchemaGap::NotInSchema` when the release has no such attribute (IFC2X3),
  instead of a `None` that reads as "authored empty".

### Unchanged

- IFC4 models, and models with no `FILE_SCHEMA` (every in-memory model),
  read exactly as in 0.2.0. Every existing test passes unmodified.
- The bulk readers keep their signatures and have no error channel. A
  header they cannot bind (none, several, or IFC4X3) reads against IFC4, as
  in 0.2.0. Call `schema_of` first to refuse such a model.
- Authoring stays IFC4-only.

### Verified

- On three real IFC2X3 ventilation models (2 systems each), six other
  IFC2X3/IFC4 models, and an IFC4 MEP model with 16 systems, 11,666 ports,
  and 5,833 connections, the results are identical before and after. None of
  the local IFC2X3 files contains a zone, electrical circuit, or port, so the
  IFC2X3-specific paths are proven by the fixture tests only.

## [0.2.0] - 2026-09-22

First release under per-crate versioning. See the
[repository changelog](../CHANGELOG.md) for the family-wide history
that produced this version.

[Unreleased]: https://github.com/openbimrs/ifc/compare/ifc-systems-v0.2.1...HEAD
[0.2.1]: https://github.com/openbimrs/ifc/releases/tag/ifc-systems-v0.2.1
[0.2.0]: https://github.com/openbimrs/ifc/releases/tag/v0.2.0
