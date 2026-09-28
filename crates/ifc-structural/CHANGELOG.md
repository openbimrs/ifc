# Changelog -- ifc-structural

All notable changes to the `ifc-structural` crate are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate follows Semantic Versioning independently of its siblings:
a release here does not imply a release of any other crate in the family.

Changes up to and including 0.2.0 are recorded family-wide in the
[repository changelog](../../CHANGELOG.md); that file is the archive for
everything released before per-crate changelogs began.

## [Unreleased]

### Changed (breaking)

- `MemberConnection` and `ActivityAssignment` are `#[non_exhaustive]`.

### Changed

- Depends on `ifc-schema` with its default features named explicitly
  (every bundled release), now that the workspace dependency turns them
  off for the facade's per-release features (#112).
- A model whose header declares `IFC4X1` or `IFC4X2` is refused with the
  existing unsupported-schema error. `ifc-schema` now bundles both
  releases, but no layout here is verified against them, so they are
  never read as IFC4 or IFC4X3.
- Enumeration checks treat a type declaration form `ifc-schema` adds later
  as not matching; follows `ifc_schema::TypeKind` becoming
  `#[non_exhaustive]`.

## [0.2.1] - 2026-09-28

### Added

- `stage_boundary_condition_in(tx, schema, kind, draft)` (#200, #201). It
  lays the record out by `schema`'s attribute names (IFC2X3 still says
  `LinearStiffness...`) and writes each stiffness in the form its declared
  type requires there: bare in IFC2X3, whose stiffnesses are plain
  measures, and the typed parameter of the SELECT member in IFC4 and IFC4X3.
  A boolean where the release admits none (every IFC2X3 stiffness) is
  refused with `InvalidDraftValue`. It takes `&Schema` like the crate's
  other writers.

### Fixed

- `stage_boundary_condition` writes the IFC4/IFC4X3 form correctly
  (#200, #201):
  - an edge condition's translational stiffness is
    `IFCMODULUSOFLINEARSUBGRADEREACTIONMEASURE(..)`, the member of
    `IfcModulusOfTranslationalSubgradeReactionSelect`, instead of
    `IFCMODULUSOFTRANSLATIONALSUBGRADEREACTIONMEASURE`, which no release
    declares;
  - warping is `IFCWARPINGMOMENTMEASURE(..)`, the member of
    `IfcWarpingStiffnessSelect`, instead of `IFCROTATIONALSTIFFNESSMEASURE`;
  - a boolean stiffness is `IFCBOOLEAN(.T.)` instead of a bare `.T.`, which
    does not say which SELECT member it is.

  It is now `stage_boundary_condition_in` with the bundled IFC4 table, so it
  is still not correct in IFC2X3; its docs point IFC2X3 callers to the new
  writer. The readers accept a bare and a typed value, as before.

## [0.2.0] - 2026-09-22

First release under per-crate versioning. See the
[repository changelog](../../CHANGELOG.md) for the family-wide history
that produced this version.

[Unreleased]: https://github.com/openbimrs/ifc/compare/ifc-structural-v0.2.1...HEAD
[0.2.1]: https://github.com/openbimrs/ifc/releases/tag/ifc-structural-v0.2.1
[0.2.0]: https://github.com/openbimrs/ifc/releases/tag/v0.2.0
