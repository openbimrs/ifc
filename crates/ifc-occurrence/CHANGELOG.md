# Changelog -- ifc-occurrence

All notable changes to the `ifc-occurrence` crate are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate follows Semantic Versioning independently of its siblings:
a release here does not imply a release of any other crate in the family.

Changes up to and including 0.2.0 are recorded family-wide in the
[repository changelog](../../CHANGELOG.md); that file is the archive for
everything released before per-crate changelogs began.

## [Unreleased]

## [0.3.0] - 2026-09-29

### Added

- `OccurrenceDraft` fields for what IFC2X3 TC1 requires of a few classes
  (#214): `shape_type` (`IfcRamp`, `IfcRoof`, `IfcStair`), `nominal_diameter`
  and `cross_section_area` (`IfcReinforcingBar`, `IfcTendon`), `bar_role`
  (`IfcReinforcingBar`), and `longitudinal_bars` and `transverse_bars`
  (`IfcReinforcingMesh`), each a new `MeshBars` of nominal diameter,
  cross-section area and spacing. These IFC2X3 records can now be authored
  with `create_with_owner_history`; IFC4 and IFC4X3, which declare the
  measures `OPTIONAL`, write them when given. A value for an attribute the
  bound release does not declare on the class is refused with
  `AuthoringNotInSchema`.
- `OccurrenceError::UnknownToken` (a `ShapeType` or `BarRole` outside the
  release's enumeration), `InvalidMeasure` (a non-positive or non-finite
  `IfcPositiveLengthMeasure`, a non-finite `IfcAreaMeasure`) and
  `TypeClassNotInSchema` (the bound release pairs no type class with the
  occurrence, such as an IFC2X3 `IfcStair`), appended.
- `OccurrenceError` implements `Display` and `std::error::Error`.
- `OccurrenceDraft::new` and one builder setter per field, named after it.
- `Occurrence::ifc4_type_class` and `Occurrence::ifc2x3_type_class`: the type
  class IFC4 ADD2 TC1 and IFC2X3 TC1 pair with each class, generated from
  their EXPRESS sources by `scripts/gen-occurrences.py`.

### Fixed

- The occurrence-to-type pairing follows the declared release (#214). It was
  IFC4X3's `CorrectTypeAssigned` in every release, so an IFC2X3 `IfcDoor`
  typed by an `IfcDoorStyle` was refused; IFC2X3 now pairs doors and windows
  with `IfcDoorStyle` and `IfcWindowStyle` and every other class with the
  later releases' type class where IFC2X3 declares it, and IFC4 uses its own
  rules (IFC4's `IfcTransformer` rule names the undeclared
  `IFCTRANFORMERTYPE`, an erratum recorded as written). The referenced type
  is compared with `TYPEOF` semantics, subtypes included.

### Changed (breaking)

- `OccurrenceDraft` is `#[non_exhaustive]`: struct literals outside the crate
  no longer compile. Use `new()` (or `default()`) and the setters; the fields
  stay public.
- `Occurrence`, the generated catalogue row, is `#[non_exhaustive]` and has
  two new fields; it can no longer be built by struct literal outside the
  crate (use the generated constants).
- An IFC2X3 or IFC4 `typed_by` is checked against that release's pairing:
  a class IFC2X3 pairs with nothing (its type class undeclared there) is
  refused with `TypeClassNotInSchema` where it was checked against the
  IFC4X3 class, and `WrongTypeClass.expected` names the release's class.

### Changed

- Depends on `ifc-schema` with its default features named explicitly
  (every bundled release), now that the workspace dependency turns them
  off for the facade's per-release features (#112).
- A model whose header declares `IFC4X1` or `IFC4X2` is refused with the
  existing unsupported-schema error. `ifc-schema` now bundles both
  releases, but no layout here is verified against them, so they are
  never read as IFC4 or IFC4X3.

## [0.2.1] - 2026-09-28

### Added

- `create_with_owner_history` (#202). It takes a caller-supplied
  `IfcOwnerHistory` id, which IFC2X3 requires on every `IfcRoot`. The id
  must be in the model or staged on the transaction and must be an
  `IfcOwnerHistory`: a missing one is refused with
  `OccurrenceError::UnresolvedOwnerHistory`, another entity with
  `OccurrenceError::NotAnOwnerHistory`. None is ever invented. This follows
  `ifc-material` (#77) and `ifc-properties` (#191).
- `OccurrenceError::MultipleSchemas`, `UnsupportedSchema`,
  `EntityNotInSchema`, `AuthoringNotInSchema`, `AuthoringRequired`,
  `UnresolvedOwnerHistory` and `NotAnOwnerHistory` (#202), appended.
  `OccurrenceError` is `#[non_exhaustive]`, so this is not breaking.
- `ifc-schema` is now a dependency: the declared release's table lays out
  each record.

### Fixed

- `create` writes the model's declared release instead of the IFC4X3
  catalogue row's layout (#202). It wrote `OwnerHistory` `$`, mandatory in
  IFC2X3, and the IFC4X3 arity, `PredefinedType` slot and tokens into
  every model. It now binds the release (a header without `FILE_SCHEMA`
  binds IFC4), lays the record out by attribute name from that release's
  table, and checks `predefined_type` against that release's enumeration.
  IFC4X3 output is unchanged for every catalogue row, and IFC4 output for
  every row IFC4 declares with the same layout (both are tested).
  **Behaviour changes:** an IFC2X3 model is refused with
  `AuthoringRequired { attribute: "OwnerHistory", .. }`; use
  `create_with_owner_history` there. A class the bound release does not
  declare is refused with `EntityNotInSchema`: IFC4X3-only classes such as
  `IfcBorehole` are refused in an IFC4 model **and in a model whose header
  declares no schema**, which binds IFC4; declare `IFC4X3` (or
  `IFC4X3_ADD2`) to author them. A token only IFC4X3 declares is refused in
  IFC4 with `UnknownPredefinedType`, and IFC2X3 classes without a
  `PredefinedType` (`IfcDoor`, `IfcWall`) refuse one with
  `NoPredefinedType`. A required attribute the draft cannot carry is
  refused with `AuthoringRequired`, not written `$`: in IFC2X3,
  `IfcRamp`/`IfcRoof`/`IfcStair.ShapeType`,
  `IfcReinforcingBar`/`IfcTendon.NominalDiameter` and
  `IfcReinforcingMesh.LongitudinalBarNominalDiameter`. Several or unknown
  `FILE_SCHEMA` tokens are refused with `MultipleSchemas` or
  `UnsupportedSchema`. The type pairing is still the IFC4X3
  `CorrectTypeAssigned` class in every release, so an IFC2X3
  `IfcDoorStyle` is not accepted as `typed_by`.

## [0.2.0] - 2026-09-22

First release under per-crate versioning. See the
[repository changelog](../../CHANGELOG.md) for the family-wide history
that produced this version.

[Unreleased]: https://github.com/openbimrs/ifc/compare/ifc-occurrence-v0.3.0...HEAD
[0.3.0]: https://github.com/openbimrs/ifc/releases/tag/ifc-occurrence-v0.3.0
[0.2.1]: https://github.com/openbimrs/ifc/releases/tag/ifc-occurrence-v0.2.1
[0.2.0]: https://github.com/openbimrs/ifc/releases/tag/v0.2.0
