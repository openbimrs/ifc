# Changelog -- ifc-validate

All notable changes to the `ifc-validate` crate are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate follows Semantic Versioning independently of its siblings:
a release here does not imply a release of any other crate in the family.

Changes up to and including 0.2.0 are recorded family-wide in the
[repository changelog](../../CHANGELOG.md); that file is the archive for
everything released before per-crate changelogs began.

## [Unreleased]

### Changed (breaking)

- Release features `ifc2x3`, `ifc4`, `ifc4x1`, `ifc4x2` and `ifc4x3`, all
  default, forward to `ifc-schema`, which this crate now depends on without
  its default features (#306). Each links one release's table, so a build
  naming only `ifc4` no longer carries the other four: an IFC4-only browser
  module with validation shrinks from 1,518,692 to 926,023 bytes. The
  default build links every release, as before. `ifc4` alone used to link
  every release too; it now links IFC4's, and `validate_declared` refuses
  a file declaring another release with `UnbundledSchema`. Under 0.x, a
  minor release.
- `validate_declared` is no longer gated on the `ifc4` feature: it exists
  in every build and refuses a release the build does not bundle.

## [0.5.0] - 2026-10-02

### Added

- Native WHERE rules over property sets and type assignments (#215),
  registered only for the releases whose EXPRESS states them and checked
  against it by `tests/registry_scope.rs`:
  - `IfcObject.UniquePropertySetNames` (IFC4, IFC4X3):
    `IfcUniqueDefinitionNames(IsDefinedBy)`, with the inverse rebuilt from
    every `IfcRelDefinesByProperties` and an `IfcPropertySetDefinitionSet`
    opened;
  - `IfcTypeObject.UniquePropertySetNames` (IFC4, IFC4X3):
    `IfcUniquePropertySetNames(HasPropertySets)`;
  - `IfcTypeProduct.ApplicableOccurrence` (IFC4, IFC4X3) and its IFC2X3
    label `IfcTypeProduct.WR41`: every object a type product is assigned
    to by `IfcRelDefinesByType` is an `IfcProduct`.

  One finding per shared name; a definition the file lacks is an
  evaluation error. IFC2X3 states no unique-set-name rule. The textual
  `IfcTypeObject.ApplicableOccurrence` and
  `IfcPropertySetTemplate.ApplicableEntity` are stated as a rule by no
  release and are not checked.
- `type_check::check_value_all`: every independent mismatch of one value
  against one declared type.

### Changed

- `type_check::attribute_types` reports every independent violation in a
  slot instead of the first (#215): each bad member of an aggregate, at
  every nesting level, and a wrapper's form together with a bad parameter
  inside it (`IFCLABEL(12)` in an `IfcLabel` slot is now both
  `type.typed.outside_select` and `type.scalar.mismatch`; a wrapper outside
  its SELECT is `type.select.member` and its parameter is still judged).
  Identical mismatches in one slot are reported once. A report on a
  malformed file can therefore hold more findings than before.
  `type_check::check_value` still returns the first.

## [0.4.0] - 2026-09-29

### Added

- Aggregate checks from the schema's bounds (#111): a level outside its
  declared size (`structure.aggregate.too_few`,
  `structure.aggregate.too_many`; `ARRAY [l:u]` needs exactly u-l+1), an
  inner level of a nested aggregate that is not an aggregate
  (`structure.aggregate.nesting`), and a repeated element in a `SET` or
  `UNIQUE` level (`structure.aggregate.duplicate`). The members of a
  `LIST OF LIST` are now type-checked against the innermost element type
  (#215).
- Every `UNIQUE` clause of the declared release is checked across the
  declaring entity and its subtypes (`structure.unique.violation`), except
  `IfcRoot.UR1`, which stays `global.UniqueGlobalId`.

### Changed (breaking)

- `structure::duplicate_global_ids` and its rule id
  `structure.unique.duplicate_global_id` are removed: the function
  duplicated `global.UniqueGlobalId` and `validate` never ran it. Its
  module now checks the release's UNIQUE clauses (`structure::unique_rules`,
  run by `validate`).
- No registered rule claims to need aggregate bounds any more:
  `IfcPolyLoop.WR21` and `IfcPolyLoop.AllPointsSameDim` are unsupported
  for needing an expression evaluator.
- `Support` is `#[non_exhaustive]`: a match needs a wildcard arm.
- `Finding` is `#[non_exhaustive]`; it can no longer be built with a struct
  literal outside the crate.
- `Path` is `#[non_exhaustive]`, so a later release can name a new location
  kind; a match needs a wildcard arm.

### Changed

- Depends on `ifc-schema` with its default features named explicitly
  (every bundled release), now that the workspace dependency turns them
  off for the facade's per-release features (#112).
- `validate_declared` validates IFC4X1 and IFC4X2 files against their own
  bundled tables instead of refusing them as unknown. No WHERE rule is
  registered for either release yet, so their report carries one
  `where.release` finding (severity `Unsupported`) saying WHERE rules were
  not evaluated, rather than reading as if they passed.
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

[Unreleased]: https://github.com/openbimrs/ifc/compare/ifc-validate-v0.5.0...HEAD
[0.5.0]: https://github.com/openbimrs/ifc/releases/tag/ifc-validate-v0.5.0
[0.4.0]: https://github.com/openbimrs/ifc/releases/tag/ifc-validate-v0.4.0
[0.3.1]: https://github.com/openbimrs/ifc/releases/tag/ifc-validate-v0.3.1
[0.3.0]: https://github.com/openbimrs/ifc/releases/tag/ifc-validate-v0.3.0
[0.2.0]: https://github.com/openbimrs/ifc/releases/tag/v0.2.0
