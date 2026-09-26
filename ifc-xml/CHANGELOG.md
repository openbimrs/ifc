# Changelog -- ifc-xml

All notable changes to the `ifc-xml` crate are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate follows Semantic Versioning independently of its siblings:
a release here does not imply a release of any other crate in the family.

Changes up to and including 0.2.0 are recorded family-wide in the
[repository changelog](../CHANGELOG.md); that file is the archive for
everything released before per-crate changelogs began.

## [Unreleased]

### Fixed

- A codec built with `XmlCodec::with_schema` now reads its own output back
  in slot order. Scalars are written as XML attributes and structured values
  as child elements, and the reader kept that document order, so
  `IfcLocalPlacement($, #2)` came back as `(#2, $)`. Schema attribute names
  now resolve to slots through the same schema; found by the corpus round
  trip (#118).
- Whitespace in element text is data: a padded string, enum or binary inside
  a list or typed wrapper no longer loses leading or trailing whitespace
  (#116).
- A string such as `" i7"` no longer reads back as a reference. The writer
  now decides attribute versus element by running the reader's own
  inference, so the two cannot disagree (#116).
- An attribute literal that overflows to infinity, such as `1e999`, reads as
  text instead of an infinite real, matching the explicit `kind="real"` path
  and the writer, which both refuse non-finite reals (#116).
- `kind="logical"` text other than `true`, `false` or `unknown` is an
  `InvalidScalar` error instead of silently becoming unknown (#116).
- Tab, line feed and carriage return are written as character references,
  which conforming XML parsers do not normalize away, and a typed wrapper's
  `type` attribute is escaped (#116).

### Added

- `XmlError::DuplicateSlot` and `XmlError::MissingSlot`: two values for one
  attribute slot, or a positional `a<i>` name that skips slots without a
  schema declaring them, are refused with the entity path instead of shifting
  later attributes. With a schema, an omitted attribute reads as unset.
- A seeded property test of the scalar contract and a differential
  STEP -> ifcXML -> Model -> STEP round trip over every committed fixture, in
  positional, schema-named and strict-profile configurations (#116, #118).

## [0.2.0] - 2026-09-22

First release under per-crate versioning. See the
[repository changelog](../CHANGELOG.md) for the family-wide history
that produced this version.

[Unreleased]: https://github.com/openbimrs/ifc/compare/ifc-xml-v0.2.0...HEAD
[0.2.0]: https://github.com/openbimrs/ifc/releases/tag/v0.2.0
