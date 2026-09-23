# Changelog -- ifc-model

All notable changes to the `ifc-model` crate are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate follows Semantic Versioning independently of its siblings:
a release here does not imply a release of any other crate in the family.

Changes up to and including 0.2.0 are recorded family-wide in the
[repository changelog](../CHANGELOG.md); that file is the archive for
everything released before per-crate changelogs began.

## [Unreleased]

### Fixed

- Builds for `wasm32-unknown-unknown` (#34). `ahash`'s default
  `runtime-rng` pulled in getrandom 0.3, which fails on that target, so no
  crate depending on this one could be compiled to WebAssembly. Native
  builds keep runtime-seeded hashing; wasm32 builds use a compile-time seed.

## [0.2.1] - 2026-09-23

### Changed

- `Model::ids_of_type` docs now point to `ifc::ids_of_type_including_subtypes`
  for subtype-inclusive queries, instead of wrongly saying `ifc-schema`
  provides them.

## [0.2.0] - 2026-09-22

First release under per-crate versioning. See the
[repository changelog](../CHANGELOG.md) for the family-wide history
that produced this version.

[Unreleased]: https://github.com/openbimrs/ifc/compare/ifc-model-v0.2.1...HEAD
[0.2.1]: https://github.com/openbimrs/ifc/releases/tag/ifc-model-v0.2.1
[0.2.0]: https://github.com/openbimrs/ifc/releases/tag/v0.2.0
