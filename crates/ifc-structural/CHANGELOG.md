# Changelog -- ifc-structural

All notable changes to the `ifc-structural` crate are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate follows Semantic Versioning independently of its siblings:
a release here does not imply a release of any other crate in the family.

Changes up to and including 0.2.0 are recorded family-wide in the
[repository changelog](../../CHANGELOG.md); that file is the archive for
everything released before per-crate changelogs began.

## [Unreleased]

## [0.4.0] - 2026-10-02

### Changed (breaking)

- `LoadKind` is `#[non_exhaustive]` and gains `SingleForceWarping`,
  `SingleDisplacement` and `SingleDisplacementDistortion` (#228). An
  exhaustive `match` on it no longer compiles; add a wildcard arm.

### Added

- IFC2X3 varying linear and planar actions (#229).
  `StructuralAction::is_varying`, `varying_applied_load_location` and
  `subsequent_applied_loads` read `IfcStructuralLinearActionVarying` and
  `IfcStructuralPlanarActionVarying`; the loads come back in list order as
  `StaticLoad`, the `LIST [1:?]` / `LIST [2:?]` minimums are enforced with
  `InvalidCardinality`, and all three return `None`/`false` under IFC4 and
  IFC4X3, which declare neither entity. `ActionDraft::varying` with the new
  `VaryingActionDraft` stages either subtype from a `Linear` or `Planar`
  kind; outside IFC2X3 it refuses with the new
  `StructuralError::EntityNotInSchema`, and a short list, another kind or a
  wrong reference refuses before anything is staged.
- `StructuralView::surface_reinforcement_area` and the
  `SurfaceReinforcementArea` projection for `IfcSurfaceReinforcementArea`
  (IFC4, IFC4X3), which `stage_load` already authored (#228). It enforces
  `SurfaceAndOrShearAreaSpecified`, `NonnegativeArea1..3` and the
  `LIST [2:3]` bounds; an IFC2X3 view refuses it with `UnsupportedSchema`.

### Fixed

- `StructuralView::load` / `static_load` read
  `IfcStructuralLoadSingleDisplacement`,
  `IfcStructuralLoadSingleDisplacementDistortion` and
  `IfcStructuralLoadSingleForceWarping` instead of refusing them with
  `WrongType` (#228). Classification uses `Schema::is_a`, most specific
  subtype first, and `components()` returns the displacement and rotation
  slots, with `Distortion` or `WarpingMoment` appended for the subtypes.

## [0.3.0] - 2026-09-29

### Changed (breaking)

- `MemberConnection` and `ActivityAssignment` are `#[non_exhaustive]`.
- Every public draft is `#[non_exhaustive]`, so struct literals no longer
  compile outside the crate. Each gains a constructor taking its required
  fields and one builder setter per other field, named after the field and
  taking the unwrapped value (`.name("Frame")` sets `Some`; `String` fields
  take `impl Into<String>`):
  - `AnalysisModelDraft::new(global_id, predefined_type)`
  - `StructuralRootDraft::new(global_id)`
  - `RelationshipRootDraft::new(global_id)`
  - `MemberDraft::new(root, kind)`, `ConnectionDraft::new(root, kind)`
  - `ActionDraft::new(root, applied_load, coordinate_system, kind)`
  - `ReactionDraft::new(root, applied_load, coordinate_system, kind)`
  - `LoadGroupDraft::new(global_id, action_type, action_source, kind)`
  - `ResultGroupDraft::new(global_id, theory_type, is_linear)`
  - `MemberConnectionDraft::new(root, member, connection)`
  - `ActivityAssignmentDraft::new(root, relating_element, activity)`
  - `BoundaryConditionDraft::new()`

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

[Unreleased]: https://github.com/openbimrs/ifc/compare/ifc-structural-v0.4.0...HEAD
[0.4.0]: https://github.com/openbimrs/ifc/releases/tag/ifc-structural-v0.4.0
[0.3.0]: https://github.com/openbimrs/ifc/releases/tag/ifc-structural-v0.3.0
[0.2.1]: https://github.com/openbimrs/ifc/releases/tag/ifc-structural-v0.2.1
[0.2.0]: https://github.com/openbimrs/ifc/releases/tag/v0.2.0
