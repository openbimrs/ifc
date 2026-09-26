# Changelog -- ifc-step

All notable changes to the `ifc-step` crate are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate follows Semantic Versioning independently of its siblings:
a release here does not imply a release of any other crate in the family.

Changes up to and including 0.2.0 are recorded family-wide in the
[repository changelog](../CHANGELOG.md); that file is the archive for
everything released before per-crate changelogs began.

## [Unreleased]

### Changed (lazy loading)

- A strict read loads lazily (ADR 0015). Every record is framed with
  `openbim_step::scan`, parsed from its own span and checked against every
  rule the IFC conversion enforces -- in parallel on inputs of 4 MB and
  more -- but no value is built; each entity is decoded on first access by
  the same conversion the eager read uses. The model keeps the source
  bytes. On seven real IFC files (18-109 MB) opening is 2.1-4.6x faster at
  35-76% less peak memory; decoding everything afterwards on 8 threads is
  still 1.3-2.5x faster than the eager read at the same memory. Results are
  unchanged: lazy and eager models (and errors) are identical on the 2,273
  files of the parse-limits corpus, and any record that fails validation
  sends the whole file through the eager reader, so an error is always the
  eager reader's error. `StepReader::eager` keeps the old behaviour;
  recovery and reference-check options read eagerly.
- `read_path` reads the file into a buffer the model owns instead of
  memory-mapping it: a lazy model decodes from its source for its whole
  lifetime, and a mapping of a file changed meanwhile is undefined
  behaviour. The mapped read is the new `unsafe`
  `StepReader::read_path_mapped`, whose contract says so.
- `Index` is rebuilt on `openbim_step::scan` (breaking). Framing is strict
  -- junk between records, a missing `ENDSEC` or a second appended file are
  errors instead of being skipped -- and ids are found by binary search
  instead of a linear scan. `Index::scan`, `materialize` and
  `materialize_closure` return `Result`, and `Index::entity` returns
  `Result<Option<Entity>>`, where they used to swallow decode failures (a
  failed `materialize` returned an empty model). Subsets carry the file's
  header instead of a synthetic IFC4 one, and `Index::header` exposes it.
- Requires `openbim-step` 0.8.0.

### Added

- `StepReader::eager`, `StepReader::read_path_mapped` (unsafe) and
  `Codec::read_owned` for both codec types.

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
