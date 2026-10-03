# Changelog -- ifc-schema

All notable changes to the `ifc-schema` crate are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate follows Semantic Versioning independently of its siblings:
a release here does not imply a release of any other crate in the family.

Changes up to and including 0.2.0 are recorded family-wide in the
[repository changelog](../../CHANGELOG.md); that file is the archive for
everything released before per-crate changelogs began.

## [Unreleased]

## [0.3.1] - 2026-10-03

### Changed

- The `express` and `generation` features use the workspace's
  `openbim-step` pin, `=0.11.0` (was its own `=0.10.0`), shared with
  `ifc-step`, so a build with `express` links one `openbim-step` (#288).
  0.11 changed no EXPRESS extraction: the bundled tables still match the
  fetched schemas. No `openbim-step` type is part of this crate's API, and
  the default build links none.

## [0.3.0] - 2026-09-29

### Changed (breaking)

- Artifact format 3 records the facts above; format 1 and 2 artifacts
  still decode, with those facts empty.
- A nested aggregate attribute's `type_name` is its innermost element
  type (`IfcLengthMeasure` for `LIST OF LIST OF IfcLengthMeasure`), where
  the old extractor recorded the inner keyword `LIST`.
- The `express` and `generation` features use `openbim-step` `=0.10.0`
  (the runtime links none).
- One cargo feature per bundled release (#112): `ifc2x3`, `ifc4`, `ifc4x1`,
  `ifc4x2` and `ifc4x3`, all in `default`, so a default build bundles every
  release as before. Each accessor (`ifc2x3()`, `ifc4()`, ...) exists only
  with its feature. `ifc4` used to ship all bundled tables; it now ships
  IFC4 only, so a build with `default-features = false, features =
  ["ifc4"]` loses the other releases -- name them, or keep defaults.
- `for_version` returns `Result<&Schema, NotBundled>` instead of
  `Option<&Schema>`, and exists in every build. `Err(NotBundled)` means a
  recognised release whose feature is off; an unknown `FILE_SCHEMA` token
  is still `None` from `SchemaVersion::from_header_token`, so the two cases
  stay distinguishable.
- `write_structural_catalog` and `write_direct_structural_catalog` exist in
  every build and return an `io::ErrorKind::Unsupported` error wrapping
  `NotBundled` for a release that is not compiled in, instead of panicking.
- `artifact_decode_schema` and `BundledSchemaError` need the new
  `artifact` feature (enabled by every release feature) instead of `ifc4`.
- `SchemaVersion` is `#[non_exhaustive]`, derives `Hash`, and gains
  `Ifc4x1` and `Ifc4x2` (#33). A `match` on it needs a wildcard arm; a
  consumer should refuse a release it has not verified, never alias it to
  a neighbour. `write_structural_catalog` and
  `write_direct_structural_catalog` accept the new versions.
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

- Aggregate bounds, nested aggregation, INVERSE and UNIQUE (#111):
  `Attribute::aggregation` (levels outermost first, each an `Aggregation`
  with `AggregateKind`, lower and upper `Bound`, `unique`,
  `optional_elements`), `EntityDef::inverses` (`InverseAttribute`) and
  `EntityDef::unique_rules` (`UniqueRule`), with builders. Additive: the
  types were already `#[non_exhaustive]`. All five bundled tables are
  regenerated with them: 115/153/158/160/165 INVERSE and 17/4/4/4/4
  UNIQUE declarations for IFC2X3/IFC4/IFC4X1/IFC4X2/IFC4X3, pinned by
  tests.
- `NotBundled`, `SchemaVersion::is_bundled()` and
  `SchemaVersion::feature_name()`.
- IFC4X1 FINAL and IFC4X2 FINAL (#33): bundled tables
  `data/ifc4x1-final.bin` (801 entities, 400 types) and
  `data/ifc4x2-final.bin` (816 entities, 407 types), generated from the
  official EXPRESS files like the other three; accessors `ifc4x1()` and
  `ifc4x2()`; `for_version` returns them; header tokens `IFC4X1` and
  `IFC4X2` (the files' own `SCHEMA` names); release ids `IFC4X1_FINAL`
  and `IFC4X2_FINAL`. Tests pin that both counts differ from IFC4 and
  IFC4X3 and that each carries its own release's entities.
- `SchemaVersion::ALL`, every known version oldest first.
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
