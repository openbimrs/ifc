# Changelog -- ifc-material

All notable changes to the `ifc-material` crate are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate follows Semantic Versioning independently of its siblings:
a release here does not imply a release of any other crate in the family.

Changes up to and including 0.2.0 are recorded family-wide in the
[repository changelog](../CHANGELOG.md); that file is the archive for
everything released before per-crate changelogs began.

## [Unreleased]

### Added

- `MaterialView::constituent_fraction_diagnostic`: an opt-in policy check
  that an `IfcMaterialConstituentSet`'s fractions describe one whole
  (#103). It returns a `ConstituentFractionDiagnostic` when every
  constituent states a fraction but the sum is further from 1 than the
  caller's tolerance (`SumNotOne`), or when stated and missing fractions
  are mixed (`PartiallyStated`). IFC4 declares no WHERE rule on the sum,
  so this is never a decode error and never normalises: the accessors
  keep returning the authored fractions.

## [0.2.0] - 2026-09-22

First release under per-crate versioning. See the
[repository changelog](../CHANGELOG.md) for the family-wide history
that produced this version.

[Unreleased]: https://github.com/openbimrs/ifc/compare/ifc-material-v0.2.0...HEAD
[0.2.0]: https://github.com/openbimrs/ifc/releases/tag/v0.2.0
