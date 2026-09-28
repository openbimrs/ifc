# Changelog -- ifc-validate

All notable changes to the `ifc-validate` crate are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate follows Semantic Versioning independently of its siblings:
a release here does not imply a release of any other crate in the family.

Changes up to and including 0.2.0 are recorded family-wide in the
[repository changelog](../../CHANGELOG.md); that file is the archive for
everything released before per-crate changelogs began.

## [Unreleased]

### Changed

- SELECT resolution treats a type declaration form `ifc-schema` adds later
  like an undeclared member (fails closed); follows `ifc_schema::TypeKind`
  becoming `#[non_exhaustive]`.

## [0.3.1] - 2026-09-28

### Added

- The form of every value is checked against ISO 10303-21:2016 (#199): a
  typed parameter is written exactly where the declared type is a SELECT
  (§12.1.8), and the bare value everywhere else (§12.1.6, §12.1.7). Three
  new rule ids, all errors, each pinned by adversarial fixture pairs:
  - `type.typed.outside_select`: a typed parameter of the declared type,
    or of a specialisation of it, where the declared type is not a SELECT.
    `IFCAREAMEASURE(12.5)` in `IfcQuantityArea.AreaValue` is reported; it
    used to pass because the payload was judged against the wrapper alone.
  - `type.typed.wrong_type`: a typed parameter of another type there.
    `IFCLABEL('x')` in `AreaValue` used to pass as well.
  - `type.select.untyped`: a bare value that is not a reference where the
    declared type is a SELECT, such as `NominalValue : IfcValue` written
    `1.` instead of `IFCREAL(1.)`. Part 21 requires the typed form for every
    SELECT, not only ambiguous ones. A SELECT of entities alone still
    reports `type.entity.expected_reference`.

  All three apply to aggregate members against the element type, and to
  the parameter inside a typed wrapper against the wrapper's type. Whether
  the declared type is a SELECT follows defined-type aliases, as §12.1.8
  EXAMPLE 2 encodes a type aliasing a SELECT. They are errors, not warnings,
  although many readers unwrap a well-typed wrapper: the file is not legal,
  and `ifcopenshell.validate` rejects both forms. The reasoning is in the
  `type_check` module docs.
- `type_check::Mismatch` has the variants `TypedOutsideSelect`,
  `TypedWrongType` and `UntypedSelectValue`.

### Changed

- `type.select.member` walks only nested SELECTs, never the underlying
  type of a defined type in the select-list: §12.1.8 requires the keyword
  to name a type the SELECT, or a SELECT nested in it, lists. `IFCRATIOMEASURE(0.5)` in an
  `IfcColourOrFactor` slot, which lists `IfcNormalisedRatioMeasure`, is now
  reported.

## [0.3.0] - 2026-09-27

This release is **breaking** (0.2 -> 0.3): `Severity` gains a variant,
`Summary` gains a field, `RuleEntry` gains a field, `Mismatch` gains a
variant, three rule ids are renamed, and `Budget::max_depth` is removed.

### Added

- `Severity::EvaluationError` and `Finding::evaluation_error`: an
  implemented rule that applies to an instance but cannot be decided for it
  -- the schema tables lack an attribute the rule reads, an operand has a
  shape the rule cannot reason about, or a target the rule must type-test is
  absent -- is now reported under the rule's own id instead of being skipped
  silently (#115). `Summary::evaluation_errors` counts them, and
  `Report::is_conformant` is `false` while any is present. An unset (`$`)
  operand is still not an evaluation error: optional operands are guarded
  by the rules themselves and mandatory ones are `structure.required.missing`.
- Every rule id the crate emits is pinned by an adversarial pair of
  fixtures, and an inventory read from the crate's source fails the build
  when a new id ships without one (#114).
- `type.entity.expected_reference`: a value that is not an entity reference
  in a slot only a reference can fill -- an entity-typed slot, a member of an
  aggregate of entities, or a SELECT whose alternatives are all entities --
  is reported (#113). A string in `IfcRelSequence.RelatingProcess` used to
  get no structure or type finding at all.

### Changed

- **Breaking:** `Severity` is `#[non_exhaustive]` and has the new
  `EvaluationError` variant, ordered after `Error`; exhaustive matches
  outside the crate must add a wildcard arm.
- **Breaking:** `Summary` is `#[non_exhaustive]` and has the new
  `evaluation_errors` field; its `Display` now also prints that count.
- **Breaking:** `RuleEntry` has a new `releases` field, and
  `RuleEntry::applies_to` says whether a schema's release declares the rule.
  The engine now takes every rule's scope from its registry entry: it runs
  only under the releases listed there, on the entry's entity *and its
  subtypes* (#139).
- **Breaking:** rule ids now always name the entity that declares the rule,
  with the label the release uses (#139):
  `IfcRelAssignsToGroupByFactor.NoSelfReference` is
  `IfcRelAssignsToGroup.NoSelfReference`;
  `IfcPhysicalSimpleQuantity.WR21` is `IfcQuantityLength.WR21`; and
  `IfcPolyLoop.WR21` is reported only under IFC2X3, with IFC4 and IFC4X3
  reporting the same unsupported predicate as `IfcPolyLoop.AllPointsSameDim`.
- **Breaking:** `type_check::Mismatch` is `#[non_exhaustive]` and has the
  new `ExpectedReference` variant (#113). `type_check::check_value` now
  checks aggregate members against the element type, and reports a
  reference written where the declared type resolves to a primitive.
- `IfcExternalReference.WR1` reads `ItemReference` under IFC2X3 and
  `Identification` under IFC4/IFC4X3, as each release's EXPRESS declares,
  instead of whichever of the two resolved.

### Removed

- **Breaking:** `Budget::max_depth`. Nothing read it: every walk the crate
  performs is over the bundled schema's type graph, which a file cannot
  lengthen, and the SELECT walk's own bound is now proven sufficient for
  every SELECT the bundled schemas use. `Budget::max_findings` remains the
  one file-controlled limit.

### Fixed

- Native WHERE rules run on the entities and releases the schema declares
  them for (#139). `NoSelfReference` now checks plain `IfcRelAssignsToGroup`,
  which was never checked; `IfcMaterialLayer.NormalizedPriority` now checks
  `IfcMaterialLayerWithOffsets`; and
  `IfcRelDefinesByProperties.NoRelatedTypeObject` no longer runs under
  IFC2X3, which does not declare it. Unsupported rules are likewise admitted
  only under releases that declare them. A new schema-backed test checks
  every registered id, entity and release set against the normative EXPRESS.
- References and values inside aggregates and SELECT slots are type-checked
  (#113). `structure.reference.wrong_type` now judges every member of an
  aggregate of entities (a property set in `SET OF IfcProduct`), including
  aggregates reached through a type that aliases one; `type.select.member`
  now judges an entity reference against the SELECT's closure, directly and
  inside aggregates (an `IfcWall` as `RelatingMaterial`); and
  `type.scalar.mismatch` now judges aggregate members (a string in
  `Coordinates`) and a reference in a primitive slot. Nested attribute
  aggregates (`LIST OF LIST OF ...`) stay unchecked: the schema tables do
  not retain their element type. A reference to an entity whose type the
  tables do not declare -- typically a later release's entity -- is no
  longer reported as `structure.reference.wrong_type`; there is no basis for
  a subtype verdict, and `type.entity.unknown` already warns about it.
- `type.scalar.mismatch` now checks bounded and fixed-width strings. The
  primitive was read from the trailing token of the resolved type, so
  `STRING(255)` -- IFC4's `IfcLabel` and `IfcIdentifier` -- recognised
  nothing and every such slot went unchecked; an integer in a `Name` slot
  was accepted.

## [0.2.0] - 2026-09-22

First release under per-crate versioning. See the
[repository changelog](../../CHANGELOG.md) for the family-wide history
that produced this version.

[Unreleased]: https://github.com/openbimrs/ifc/compare/ifc-validate-v0.3.1...HEAD
[0.3.1]: https://github.com/openbimrs/ifc/releases/tag/ifc-validate-v0.3.1
[0.3.0]: https://github.com/openbimrs/ifc/releases/tag/ifc-validate-v0.3.0
[0.2.0]: https://github.com/openbimrs/ifc/releases/tag/v0.2.0
