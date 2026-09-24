# Changelog -- ifc-step

All notable changes to the `ifc-step` crate are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate follows Semantic Versioning independently of its siblings:
a release here does not imply a release of any other crate in the family.

Changes up to and including 0.2.0 are recorded family-wide in the
[repository changelog](../CHANGELOG.md); that file is the archive for
everything released before per-crate changelogs began.

## [Unreleased]

### Changed

- Requires `openbim-step` 0.7.0, matching `ifc-schema`. Both pin the parser
  exactly, so the pair must move together.
- Reading STEP builds the model from `openbim-step`'s borrowed events, so
  each value is allocated once, directly in its model form, instead of
  once as a parser `String` and again as the model's `Arc<str>`; records
  are consumed instead of cloned. The model is identical (checked over
  2,273 files); reading takes 22-40% fewer instructions and 13-35% fewer
  cycles on seven real IFC files, with resident memory unchanged.

## [0.2.1] - 2026-09-23

### Changed

- Requires `openbim-step` 0.5.1, matching `ifc-schema` 0.2.1. Both pin the
  parser exactly, so the pair must move together.

## [0.2.0] - 2026-09-22

First release under per-crate versioning. See the
[repository changelog](../CHANGELOG.md) for the family-wide history
that produced this version.

[Unreleased]: https://github.com/openbimrs/ifc/compare/ifc-step-v0.2.1...HEAD
[0.2.1]: https://github.com/openbimrs/ifc/releases/tag/ifc-step-v0.2.1
[0.2.0]: https://github.com/openbimrs/ifc/releases/tag/v0.2.0
