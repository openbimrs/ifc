# Changelog -- ifc-template-catalog

All notable changes to the `ifc-template-catalog` crate are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate follows Semantic Versioning independently of its siblings:
a release here does not imply a release of any other crate in the family.

Changes up to and including 0.2.0 are recorded family-wide in the
[repository changelog](../../CHANGELOG.md); that file is the archive for
everything released before per-crate changelogs began.

## [Unreleased]

### Added

- `corrected_catalog(CatalogEdition::Ifc4x3Add2)`: an IFC4X3 ADD2 corrected
  profile whose one patch, `NEH-IFC4X3-PSD-0001`, adds
  `Pset_Stationing.HasIncreasingStation` (`IfcBoolean`). The published ADD2
  documentation (6.6.4.10) lists it; the PSD XML the official snapshot is
  generated from omits it, and the official snapshot is unchanged (#216).
- `PatchOperation::AddProperty` and `PatchError::NotAPropertySet`.

### Changed (breaking)

- `ValidationIssue`, `ValidationReport`, `CatalogDiagnostic`,
  `ExportSummary`, `Advisory`, `AppliedPatch` and `UnresolvedApplicability`
  are `#[non_exhaustive]`; they can no longer be built with a struct literal
  outside the crate.

## [0.2.1] - 2026-09-27

### Fixed

- The built-in environmental advisories cite their decision record at its
  restored path, `docs/adr/0017-versioned-psd-qto-catalog.md`; the old
  `0010` path had been reassigned to an unrelated ADR.

## [0.2.0] - 2026-09-22

First release under per-crate versioning. See the
[repository changelog](../../CHANGELOG.md) for the family-wide history
that produced this version.

[Unreleased]: https://github.com/openbimrs/ifc/compare/ifc-template-catalog-v0.2.1...HEAD
[0.2.1]: https://github.com/openbimrs/ifc/releases/tag/ifc-template-catalog-v0.2.1
[0.2.0]: https://github.com/openbimrs/ifc/releases/tag/v0.2.0
