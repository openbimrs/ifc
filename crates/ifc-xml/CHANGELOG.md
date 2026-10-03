# Changelog -- ifc-xml

All notable changes to the `ifc-xml` crate are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate follows Semantic Versioning independently of its siblings:
a release here does not imply a release of any other crate in the family.

Changes up to and including 0.2.0 are recorded family-wide in the
[repository changelog](../../CHANGELOG.md); that file is the archive for
everything released before per-crate changelogs began.

## [Unreleased]

## [0.4.1] - 2026-10-03

### Added

- `XmlCodec::xsd` now writes the buildingSMART XSD configuration too (#274).
  Every entity is a top-level element named as the schema spells it
  (`IfcWall`) with `id="i<n>"`; entity values are `ref` references with
  `xsi:nil`, and `xsi:type` where the entity is a subtype of the declared
  type; simple values are XML attributes in their XSD lexical form (lower
  case enumerations, `xs:double` reals with a '.', `xs:hexBinary`), lists
  whitespace separated; SELECTs hold entity elements or `-wrapper` values;
  aggregates are containers of items, `Seq-` wrapped inner lists, or flat
  items with `ifc:arraySize`; the 21 attributes the configuration leaves off
  a relationship (`IfcRelAggregates.RelatingObject`) are written as
  references inside the inverse of the entity they name (`IsDecomposedBy`);
  the header is the XSD's. Each attribute's form comes from the derivation
  the reader uses, now one shared rule plus the configuration's few
  exceptions, and a schema-backed test checks every attribute, inverse and
  element order against both release XSDs. Every IFC4 fixture the writer
  accepts validates against `IFC4.xsd` with no error and reads back with the
  XSD reader to the same model (entities numbered in model order, `SET`s
  filled through inverses in document order); the STEP implementation level
  has no ifcXML field and is not written.
- `XmlError::Unrepresentable`: the writer refuses a model the configuration
  cannot carry exactly rather than write a different model or an invalid
  document -- an integer where a real is declared, a string with a tab or
  line break (the XSD types strings as `xs:normalizedString`) or over its
  declared width, a string with whitespace in a list attribute, a
  partial-byte binary, an unset mandatory element, an aggregate outside its
  declared bounds, more than one header author, organization or
  description, `IfcRelDefinesByObject.RelatingObject` (the configuration
  writes no inverse for it), a second relationship where the XSD allows one
  (`HasOpenings`), or an inverse the entity's XSD type restricts away
  (IFC4 `IfcOrientedEdge.StyledByItem`). Models the schema does not describe
  are refused with the existing typed errors.

### Fixed

- The XSD reader accepts the namespace-qualified `ifc:arraySize`,
  `ifc:itemType` and `ifc:cType` a valid document carries: the XSD declares
  them as global attributes, so unqualified ones are not valid. It refused
  them as undeclared; unqualified ones are still read.

## [0.4.0] - 2026-10-02

### Changed (breaking)

- A codec built with `XmlCodec::with_schema` or
  `XmlCodec::with_schema_and_profile` now reads strictly
  (`SchemaReading::Strict`, #266). Every value is typed from its attribute's
  declaration instead of inferred from its text: `Name="1"` reads as the
  label `'1'` (it read as the integer `1`), an enumeration's text is checked
  against its members, a `kind` the declaration does not admit (an integer
  in a label, a bare value in a SELECT, a typed value outside it) is refused.
  An XML attribute or child element the entity does not declare is refused
  with `XmlError::UnknownAttribute`, naming entity, element and attribute;
  it was placed after the declared slots. An entity the schema does not
  declare, an abstract one, and a reference to a missing entity or one of a
  type the declaration does not admit are refused too. The old read
  silently misread such content, so the safe read is now the default;
  `.with_reading(SchemaReading::Lenient)` restores it, and the codec without
  a schema is unchanged. Unknown entities and attributes still round-trip
  through the schema-less codec and the lenient read.
- The schema-aware writer writes a value as an XML attribute only where the
  strict read types its text back to the same value; elsewhere it uses the
  explicit `kind` element, which the lenient read accepts and the strict
  read refuses unless the declaration admits it. Output for models the
  schema describes is unchanged.

### Added

- `XmlCodec::xsd` and `XmlLayout::Xsd`: a reader of the buildingSMART ifcXML
  configuration the release XSD declares (#265), for IFC4 ADD2 TC1 and IFC4X3
  ADD2. Entities nested and defined in place, `ref`/`href` references with
  `xsi:nil`, `xsi:type`, inverse attributes (which supply the attribute they
  invert), space-separated and flattened list attributes, `-wrapper` and
  `Seq-...-wrapper` values, and enumeration, boolean, logical and binary text
  typed from the schema, read into the same model as the STEP form: entity
  type, typed-parameter and enumeration names upper-case, entities numbered
  in document order. Refused with a typed error rather than approximated: a
  name the entity does not declare, a value its declaration does not admit
  (including decimal commas), a flattened nested aggregate whose inner sizes
  neither the schema nor an `arraySize` fixes, an inverse contradicting its
  relationship or implying a position in an ordered list, `pos`/`path`
  addressing, external `href`s, partial-byte binaries. A value for a slot a
  subtype redeclares DERIVE, which the XSD still admits as an XML attribute,
  is typed and read as derived. The layout is read only; writing it is
  refused.
- `XmlProfile::Ifc4x3Add2`, `XmlProfile::namespaces` (IFC4 ADD2 TC1 also
  accepts the `http://www.buildingsmart-tech.org/ifcXML/IFC4/Add2` namespace
  its own Annex E examples declare, in the XSD layout only) and
  `XmlProfile::version`.
- `SchemaReading`, `XmlCodec::with_reading`, `XmlCodec::reading` and
  `XmlCodec::layout`.
- `XmlError` variants `UnknownEntity`, `AbstractEntity`, `UnknownAttribute`,
  `TypeMismatch`, `WrongForm`, `UnresolvedReference`, `DuplicateId`,
  `InverseConflict`, `SchemaMismatch` and `Unsupported`.
- `XmlCodec` implements `Debug` and `Clone`; `XmlError` implements `Clone`.
- A schema-backed test checks the XSD reader's configuration against every
  entity type of both release XSDs, which `scripts/fetch-ifc-schemas.sh` now
  fetches (checksummed, never committed). An env-gated test compares paired
  STEP and ifcXML files (`tests/xsd_corpus.rs` documents how to run it).
- `tests/xsd_output.rs`, an opt-in (`--ignored`) check that writes every IFC4
  and IFC4X3 fixture with the strict profile and validates it with `xmllint`
  against the fetched release XSD (#117). It fails on malformed output, a
  namespace that is not the XSD's `targetNamespace`, a control document the
  XSD or `XmlCodec::xsd` refuses, and any XSD error beyond the documented
  native-layout departures. The published `IFC4X3_ADD2.xsd` does not compile
  in libxml2 or Xerces, which the check records. How to run it is in the
  README.

### Security

- Require `quick-xml` 0.42 (was 0.37), which fixes RUSTSEC-2026-0194
  (quadratic duplicate-attribute check on one start tag) and
  RUSTSEC-2026-0195 (unbounded namespace-declaration allocation in
  `NsReader`); both are denial of service on untrusted input (#267).

### Changed

- `impl From<quick_xml::Error> for XmlError` is kept, but its source type is
  now quick-xml 0.42's `Error`. Code that converts a quick-xml 0.37 error
  into `XmlError` must upgrade quick-xml too. Values read are unchanged:
  entity and character references are still resolved, and literal tabs and
  line breaks in attribute values and text are still kept rather than
  normalised to spaces; a regression test pins this.

### Documentation

- Strict-profile output is documented as what it is: the crate's own layout
  under the release XSD's target namespace and schema token, not the XSD
  configuration, and not valid against the release XSD (#117). `XmlProfile`
  no longer calls the XSD "bundled"; it is fetched, never shipped.

## [0.3.0] - 2026-09-29

## [0.2.1] - 2026-09-27

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
[repository changelog](../../CHANGELOG.md) for the family-wide history
that produced this version.

[Unreleased]: https://github.com/openbimrs/ifc/compare/ifc-xml-v0.4.1...HEAD
[0.4.1]: https://github.com/openbimrs/ifc/releases/tag/ifc-xml-v0.4.1
[0.4.0]: https://github.com/openbimrs/ifc/releases/tag/ifc-xml-v0.4.0
[0.3.0]: https://github.com/openbimrs/ifc/releases/tag/ifc-xml-v0.3.0
[0.2.1]: https://github.com/openbimrs/ifc/releases/tag/ifc-xml-v0.2.1
[0.2.0]: https://github.com/openbimrs/ifc/releases/tag/v0.2.0
