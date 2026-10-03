# Changelog -- ifc-template-catalog

All notable changes to the `ifc-template-catalog` crate are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate follows Semantic Versioning independently of its siblings:
a release here does not imply a release of any other crate in the family.

Changes up to and including 0.2.0 are recorded family-wide in the
[repository changelog](../../CHANGELOG.md); that file is the archive for
everything released before per-crate changelogs began.

## [Unreleased]

### Changed (#317)

- The embedded snapshots are one container, `data/catalog.bin` (snapshot
  format 3), instead of three bincode files: 1,423,604 bytes instead of
  3,648,573 (547,657 instead of 966,507 under `gzip -9`). Strings used more
  than once are stored once in a table, strings used once stay inline, and
  each distinct property, quantity and set record is stored once across
  editions; each edition keeps its manifest, source digest and
  per-template provenance. Lossless: every edition decodes, in the official
  and the corrected profile, to exactly the catalogs format 2 did, which a
  test asserted against the old files before they were removed; content
  fingerprints taken from them stay pinned in `tests/snapshot.rs`.
- The first lookup of an edition decodes that edition only.
- The generator (`ifc-template-catalog-generate`) writes one edition into
  the container and keeps the others; the hidden `generation` module and
  the format-2 codec are gone.

### Added (#317, #318)

- Module `snapshot`: `encode` (a container or a per-edition file,
  deterministic), `decode_edition`, `decode_all`, `editions`, `file_name`,
  the pins `CONTAINER_SHA256` and `pinned_sha256`, `EDITIONS`,
  `FORMAT_VERSION` and `EncodeError`. `ArchiveError` is public there and
  gains `MissingEdition`.
- Feature `runtime`, module `runtime`: `install` an edition from its
  per-edition file or the container, checked against the pinned SHA-256
  (`RuntimeCatalogError::DigestMismatch` otherwise), then `load_catalog`
  in the official or corrected profile, `is_installed` and
  `decode_verified`; an edition not installed is
  `RuntimeCatalogError::NotInstalled`. Builds for wasm32; `embedded` stays
  the default.
- Example `export_snapshots` writes the per-edition files and checks each
  pin; the npm package ships them.

### Semver

- A new public module, feature and error variant, and a changed data
  layout under `data/`: a minor release (0.4.0). The API callers used
  (`embedded::*`, `Catalog`, the definitions) is unchanged.

## [0.3.1] - 2026-10-02

### Security

- With the `xml` feature, require `quick-xml` 0.42 (was 0.37), which fixes
  RUSTSEC-2026-0194 (quadratic duplicate-attribute check on one start tag)
  and RUSTSEC-2026-0195 (unbounded namespace-declaration allocation in
  `NsReader`); both are denial of service on untrusted input (#267). PSD and
  QTO import reads the same values as before: references are resolved, and
  literal tabs and line breaks in attributes and text are kept.

## [0.3.0] - 2026-09-29

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

[Unreleased]: https://github.com/openbimrs/ifc/compare/ifc-template-catalog-v0.3.1...HEAD
[0.3.1]: https://github.com/openbimrs/ifc/releases/tag/ifc-template-catalog-v0.3.1
[0.3.0]: https://github.com/openbimrs/ifc/releases/tag/ifc-template-catalog-v0.3.0
[0.2.1]: https://github.com/openbimrs/ifc/releases/tag/ifc-template-catalog-v0.2.1
[0.2.0]: https://github.com/openbimrs/ifc/releases/tag/v0.2.0
