# Changelog -- ifc-schema

All notable changes to the `ifc-schema` crate are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate follows Semantic Versioning independently of its siblings:
a release here does not imply a release of any other crate in the family.

Changes up to and including 0.2.0 are recorded family-wide in the
[repository changelog](../../CHANGELOG.md); that file is the archive for
everything released before per-crate changelogs began.

## [Unreleased]

### Changed (breaking)

- `ifc-schema` owns its schema types. `Attribute`, `EntityDef`, `TypeDef`,
  `TypeKind` and the newly exported `WhereRule` are defined here instead of
  re-exported from `openbim_step::express`, and all five are
  `#[non_exhaustive]`: construct them with `Attribute::new`,
  `EntityDef::new`, `TypeDef::new`, `WhereRule::new` and the builder methods
  (`with_supertype`, `with_attribute`, `with_derived`, `with_where_rule`,
  `abstract_entity`, `optional`, `aggregate`), and give every `match` on
  `TypeKind` a wildcard arm. Field names, `supertype()`, `is_derived()` and
  `is_defined()` are unchanged. Rationale: an `openbim-step` release no
  longer ripples into this crate's public API, and later facts about a
  declaration (aggregate bounds, INVERSE, UNIQUE) can be added as fields
  without another break.
- `openbim-step` is an optional dependency, linked only by the new `express`
  feature and by `generation`. The bundled tables decode straight into the
  owned types; the default build no longer links a parser.
- `Schema::from_express` and `Schema::from_express_bytes` require the new
  `express` feature.
- Removed: `Schema::from_parsed(ParsedSchema)` (use
  `Schema::new(name, entities, types)`), `Schema::graph()` (the
  `openbim_step::SchemaGraph` it returned is no longer held; `Schema`
  answers the same queries itself), and the `express` module with its
  `parse`/`ParsedSchema` re-exports (use `openbim_step::express` directly).
- `EntityDef` no longer carries `redeclared`/`is_redeclared()`. No bundled
  table ever recorded explicit redeclarations (the artifact format drops
  them), so they were always empty for `ifc2x3()`, `ifc4()` and `ifc4x3()`.
- `artifact_decode_schema` returns a `Schema` and `artifact_encode_schema`
  (`generation`) takes one, instead of `openbim_step::express::ParsedSchema`.

### Added

- `Schema::new`, `Schema::entities()` and `Schema::types()` (declarations in
  source order), `PartialEq`/`Eq` for `Schema`, and the `BundledSchemaError`
  export.

### Unchanged

- The bundled artifacts are byte-identical: regenerating all three with the
  ported generator reproduces the committed files, and `FORMAT_VERSION`
  stays 2.

### Changed

- Requires `openbim-step` 0.7.0, matching `ifc-step`. Both pin the parser
  exactly, so the pair must move together. `openbim-step` 0.6 replaced
  `EntityDef::supertype` (a field) with `supertypes` plus a `supertype()`
  accessor for multiple inheritance; IFC schemas are single-inheritance, so
  the serialized artifact is unchanged.

## [0.2.4] - 2026-09-27

Maintenance release from `maint/ifc-schema-0.2`; the code is unchanged since
0.2.3.

### Added

- A crate README, which is the crates.io page.

## [0.2.3] - 2026-09-27

Maintenance release from `maint/ifc-schema-0.2`: the 0.2.2 code with only
the data change below, so it keeps `openbim-step` 0.5.1 and every
dependent's `^0.2.2` resolves to it. `main` carries the same change.

### Changed

- The bundled IFC2X3 TC1, IFC4 ADD2 TC1 and IFC4X3 ADD2 schemas no longer
  carry WHERE-rule expressions: `EntityDef::where_rules` keeps each rule's
  `label`, and its `expression` is empty. The expressions are CC BY-ND schema
  text that nothing here evaluates. A schema parsed at run time with
  `Schema::from_express` keeps them. The artifacts are about 23-38% smaller.
  `data/NOTICE.md` now states the provenance of the bundled files.

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
[repository changelog](../../CHANGELOG.md) for the family-wide history
that produced this version.

[Unreleased]: https://github.com/openbimrs/ifc/compare/ifc-schema-v0.2.4...HEAD
[0.2.4]: https://github.com/openbimrs/ifc/releases/tag/ifc-schema-v0.2.4
[0.2.3]: https://github.com/openbimrs/ifc/releases/tag/ifc-schema-v0.2.3
[0.2.2]: https://github.com/openbimrs/ifc/releases/tag/ifc-schema-v0.2.2
[0.2.1]: https://github.com/openbimrs/ifc/releases/tag/ifc-schema-v0.2.1
[0.2.0]: https://github.com/openbimrs/ifc/releases/tag/v0.2.0
