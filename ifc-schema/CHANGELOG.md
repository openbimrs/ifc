# Changelog -- ifc-schema

All notable changes to the `ifc-schema` crate are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate follows Semantic Versioning independently of its siblings:
a release here does not imply a release of any other crate in the family.

Changes up to and including 0.2.0 are recorded family-wide in the
[repository changelog](../CHANGELOG.md); that file is the archive for
everything released before per-crate changelogs began.

## [Unreleased]

### Changed

- Requires `openbim-step` 0.7.0, matching `ifc-step`. Both pin the parser
  exactly, so the pair must move together. `openbim-step` 0.6 replaced
  `EntityDef::supertype` (a field) with `supertypes` plus a `supertype()`
  accessor for multiple inheritance; IFC schemas are single-inheritance, so
  the serialized artifact is unchanged.

## [0.2.2] - 2026-09-23

### Fixed

- Builds for `wasm32-unknown-unknown` (#34). `ahash`'s default
  `runtime-rng` pulled in getrandom 0.3, which fails on that target, so no
  crate depending on this one could be compiled to WebAssembly. Native
  builds keep runtime-seeded hashing; wasm32 builds use a compile-time seed.

## [0.2.1] - 2026-09-23

### Added

- `Schema::subtypes` and `Schema::direct_subtypes`: every entity inheriting
  from a name, the inverse of `is_a`. Checked against `is_a` for every
  ordered entity pair of all three bundled schemas (#32).

### Changed

- Requires `openbim-step` 0.5.1, which provides the downward walk.

## [0.2.0] - 2026-09-22

First release under per-crate versioning. See the
[repository changelog](../CHANGELOG.md) for the family-wide history
that produced this version.

[Unreleased]: https://github.com/openbimrs/ifc/compare/ifc-schema-v0.2.2...HEAD
[0.2.2]: https://github.com/openbimrs/ifc/releases/tag/ifc-schema-v0.2.2
[0.2.1]: https://github.com/openbimrs/ifc/releases/tag/ifc-schema-v0.2.1
[0.2.0]: https://github.com/openbimrs/ifc/releases/tag/v0.2.0
