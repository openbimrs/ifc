# Changelog -- ifc-model

All notable changes to the `ifc-model` crate are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate follows Semantic Versioning independently of its siblings:
a release here does not imply a release of any other crate in the family.

Changes up to and including 0.2.0 are recorded family-wide in the
[repository changelog](../../CHANGELOG.md); that file is the archive for
everything released before per-crate changelogs began.

## [Unreleased]

## [0.4.4] - 2026-10-29

### Fixed (#595)

- `Model::set_attribute` and `Model::set_attributes` panic before any write
  for an attribute index of `usize::MAX`, instead of wrapping in release builds
  and truncating the entity, or leaving the first edits of a batch unrecorded.

### Added (#595)

- `Model::preview` runs a change and rolls it back exactly, the revision
  included: entities, orders, header, next free id, diagnostics and the
  maintained indexes are put back, and a `History`, `Transaction` or `Inverse`
  made before it stays valid. Revisions drawn inside are never reissued.
  Semver: minor (additive).

## [0.4.3] - 2026-10-28

### Added (#587)

- `Model::reverse_index`: the reverse index, built on the first call and kept
  current by every write, undo and redo after that; equal to
  `ReverseIndex::build` at every moment.
- `Model::global_id_candidates`: the entities whose first attribute is the
  text of a `Guid`, ascending by id, kept the same way.
- `Model::drop_derived_indexes`, to free both.
- `PartialEq` and `Eq` on `ReverseIndex`.

### Changed (#587)

- `Transaction::preflight` reads the model's own reverse index when something
  built it, instead of scanning the model for a commit that removes.
- Undoing a batch is linear in the batch: file order and type buckets are
  searched from the end.

Semver: minor.

## [0.4.2] - 2026-10-28

### Changed

- Copyright holder in NOTICE and licence texts is rheonda.

## [0.4.1] - 2026-10-28

### Changed

- Licence: `AGPL-3.0-or-later OR LicenseRef-Commercial` (was `AGPL-3.0-or-later`); a commercial licence is available from the copyright holder. See LICENSING.md.

Semver: patch.

## [0.4.0] - 2026-10-27

### Added (#465)

- `Inverse`, `Restored`, `InverseError`: the recorded inverse of a run of
  model writes (`Inverse::record`, `Model::begin_recording` and
  `end_recording`). Applying one restores the model exactly (file order,
  type-index order, attribute counts, header, next free id) and returns the
  redo.
- `History` and `HistoryError`: a bounded undo and redo stack of inverses;
  `History::track` records any change as one entry.
- `Transaction::commit_reversible`, `Transaction::set_header` and
  `Transaction::header`.

### Changed (breaking) (#465)

- `Model::header_mut` bumps the revision. A `Transaction` opened before a
  `header_mut` edit is now refused at commit with
  `Conflict::StaleRevision`; set the header first, or stage it with
  `Transaction::set_header`.
- Revisions are process-unique: every change draws a new value from one
  process-wide counter, so values are increasing but no longer consecutive,
  and two models never share a revision once either was written. A
  `Transaction` or `Inverse` made against one model is refused by another.
  A clone keeps its source's revision until either side changes.
- `Model::set_attributes` with an empty edit list changes nothing and no
  longer bumps the revision.
- `Model::clone` does not copy an open recording.

Semver: the revision changes can refuse code that committed before, so this
release is 0.4.0, not a patch.

## [0.3.1] - 2026-10-02

### Fixed

- `Model::type_histogram` no longer lists a type with a count of zero
  after its last entity is removed (`remove`), retyped (`retype`) or
  replaced under the same id by another type (`insert`). The emptied
  type-index bucket is now dropped, so the histogram equals that of a
  model rebuilt from the same entities (#106). `ids_of_type` was already
  correct (an empty slice either way).

### Added

- `tests/index_coherence_property.rs`: seeded random sequences of
  `insert`, `push`, `remove`, `retype`, `set_attribute(s)` and committed,
  refused and stale transactions, checked after every step against a
  naive shadow store, a scan, a model rebuilt from scratch and an
  independent reference walk for `ReverseIndex`; a refused transaction
  must leave the model unchanged, revision included (#106). No new
  dependency: a local SplitMix64 over fixed seeds keeps it deterministic.

## [0.3.0] - 2026-09-29

### Changed

- `tests/package_architecture.rs` enumerates the allowed bridge-to-bridge
  dependencies (`BRIDGE_EDGES`: only `ifc-geometry -> ifc-alignment`)
  instead of allowing any bridge pair, and a new test requires the
  manifests to match that list exactly (#143, ADR 0003 amendment
  2026-09-28). No crate API changes.

### Changed (breaking)

- `Conflict` and `Stop` are `#[non_exhaustive]`: a match needs a wildcard
  arm, so a new commit-conflict or walk-stop reason is not a breaking
  change.
- `Applied` is `#[non_exhaustive]`: read its fields; it can no longer be
  built with a struct literal outside the crate.

## [0.2.3] - 2026-09-26

### Added

- Lazily decoded entities (ADR 0015). A codec builds a model with
  `Model::with_source(Arc<dyn EntitySource>)` and registers each entity as
  a byte span plus type name with `Model::insert_lazy`; `Model::get` decodes
  an entity on first access and keeps it, so references stay stable and
  every later access is a lookup. Type queries, ids, `len`, `contains` and
  `next_id` need no decoding. `Model::decode_all(threads)` decodes the rest
  in parallel for a consumer about to touch everything;
  `Model::decoded_len` reports progress. Editing, cloning and removal
  decode first and then behave exactly as before; clones share the source.
- `Codec::read_owned(Vec<u8>)`: read from a buffer the model may keep.
  Defaults to `read_bytes`; `read_from` now hands its buffer over.

### Fixed

- `Guid::parse` rejects a GlobalId whose leading character is not `0`–`3`
  (#62). 22 base-64 digits carry 132 bits and a UUID has 128, so a higher
  leading digit names no UUID. Before, it was accepted and `to_uuid` dropped
  the extra high bits: `0000000000000000000000` and `4000000000000000000000`
  expanded to the same UUID, and `$$$$…` came back from a round trip as
  `3$$$…`. Every accepted `Guid` now round-trips through
  `to_uuid`/`from_uuid` unchanged.
- Authoring helpers that validate through `Guid::parse` (in `ifc-spatial`,
  `ifc-systems`, `ifc-structural` and others) now refuse such ids as well.
  `ifc-resource` already did. A file that was written with one would have
  failed its own GlobalId check.

## [0.2.2] - 2026-09-23

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
[repository changelog](../../CHANGELOG.md) for the family-wide history
that produced this version.

[Unreleased]: https://github.com/openbimrs/ifc/compare/ifc-model-v0.4.4...HEAD
[0.4.4]: https://github.com/openbimrs/ifc/releases/tag/ifc-model-v0.4.4
[0.4.3]: https://github.com/openbimrs/ifc/releases/tag/ifc-model-v0.4.3
[0.4.2]: https://github.com/openbimrs/ifc/releases/tag/ifc-model-v0.4.2
[0.4.1]: https://github.com/openbimrs/ifc/releases/tag/ifc-model-v0.4.1
[0.4.0]: https://github.com/openbimrs/ifc/releases/tag/ifc-model-v0.4.0
[0.3.1]: https://github.com/openbimrs/ifc/releases/tag/ifc-model-v0.3.1
[0.3.0]: https://github.com/openbimrs/ifc/releases/tag/ifc-model-v0.3.0
[0.2.3]: https://github.com/openbimrs/ifc/releases/tag/ifc-model-v0.2.3
[0.2.2]: https://github.com/openbimrs/ifc/releases/tag/ifc-model-v0.2.2
[0.2.1]: https://github.com/openbimrs/ifc/releases/tag/ifc-model-v0.2.1
[0.2.0]: https://github.com/openbimrs/ifc/releases/tag/v0.2.0
