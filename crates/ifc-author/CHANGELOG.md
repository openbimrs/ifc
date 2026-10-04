# Changelog -- ifc-author

All notable changes to the `ifc-author` crate are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate follows Semantic Versioning independently of its siblings:
a release here does not imply a release of any other crate in the family.

Changes up to and including 0.2.0 are recorded family-wide in the
[repository changelog](../../CHANGELOG.md); that file is the archive for
everything released before per-crate changelogs began.

## [Unreleased]

### Fixed (#330)

- A built-in type with a width (`STRING(255)`, `STRING(22) FIXED`,
  `BINARY(32)`) is the shape of its keyword. IFC4 and IFC4X3 declare
  `IfcLabel = STRING(255)`, so a number set as an `IfcLabel` was accepted
  as unresolvable; it is now refused with `TypeMismatch`, as a number
  where a bare `STRING` is declared always was. A bug fix: a patch
  release.

## [0.3.0] - 2026-09-29

### Changed (breaking)

- `PersonDraft`, `OrganizationDraft`, `ApplicationDraft` and
  `OwnerHistoryDraft` are `#[non_exhaustive]` (#214), so a later release can
  add a field without another break. Struct literals no longer compile
  outside the crate: build each with `new(...)` and field-named setters,
  `PersonDraft::new()`, `OrganizationDraft::new(name)`,
  `ApplicationDraft::new(developer, version, full_name, identifier)` and
  `OwnerHistoryDraft::new(owning_user, owning_application, creation_date)`
  (then e.g. `.change_action("ADDED")`). Fields stay public.

### Changed

- Links no bundled schema table itself: every entry point takes the
  `Schema` from the caller. A consumer that used a table through this
  crate's dependency (`ifc_schema::ifc4()`) enables it on its own
  `ifc-schema` dependency (default features bundle every release).
- A type declaration form `ifc-schema` adds later resolves as unresolved
  (no refusal on shape, no form claim) instead of failing to compile;
  follows `ifc_schema::TypeKind` becoming `#[non_exhaustive]`.

## [0.2.2] - 2026-09-28

### Added

- `AuthorError::ValueForm`: a value of the declared type written in the
  form ISO 10303-21 does not use for it (#199). A typed parameter
  (`IFCAREAMEASURE(12.5)`) is refused where the declared type is not a
  SELECT, and a bare value where it is one, for scalars and for each member
  of an aggregate, in `EntityBuilder` and `EntityEditor`.

### Changed

- `EntityEditor` re-checks every slot of the projected entity, as before, so
  editing an entity whose untouched slots already hold a value in the wrong
  form is now refused with `ValueForm` until that slot is rewritten.

### Fixed

- A typed wrapper was judged against its own type only, so any wrapper
  passed in a slot whose declared type is not a SELECT (#199). `IFCLABEL('x')`
  in `IfcQuantityArea.AreaValue`, or in an entity-typed slot, is now a
  `TypeMismatch`, and so is a wrapper naming a type outside a SELECT's
  select-list, nested SELECTs included. A declared type the tables cannot
  resolve still accepts either form.

## [0.2.1] - 2026-09-27

### Fixed

- An attribute declared as a defined type that aliases an aggregate is an
  aggregate (#17). `IfcSite.RefLatitude`/`RefLongitude`
  (`IfcCompoundPlaneAngleMeasure = LIST [3:4] OF INTEGER`) were refused with
  `AggregateMismatch`, which blocked georeferencing. Their elements are now
  checked against the alias's element type.
- A slot that the entity or a supertype redeclares as `DERIVE` is written `*`
  automatically (#18). `IfcSIUnit.Dimensions` and the four derived slots of
  `IfcGeometricRepresentationSubContext` reported `MissingRequired`, so no unit
  assignment or Body/Axis subcontext could be authored. Passing
  `Value::Derived` explicitly is also accepted.

### Added

- `AuthorError::DerivedAttribute` refuses a value or `$` in a derived slot.
  `AuthorError::NotDerived` refuses `*` in a slot the schema does not derive.
  Before, `*` was accepted in any slot and the file was invalid. Both apply to
  `EntityBuilder` and `EntityEditor`.

## [0.2.0] - 2026-09-22

First release under per-crate versioning. See the
[repository changelog](../../CHANGELOG.md) for the family-wide history
that produced this version.

[Unreleased]: https://github.com/openbimrs/ifc/compare/ifc-author-v0.3.0...HEAD
[0.3.0]: https://github.com/openbimrs/ifc/releases/tag/ifc-author-v0.3.0
[0.2.2]: https://github.com/openbimrs/ifc/releases/tag/ifc-author-v0.2.2
[0.2.1]: https://github.com/openbimrs/ifc/releases/tag/ifc-author-v0.2.1
[0.2.0]: https://github.com/openbimrs/ifc/releases/tag/v0.2.0
