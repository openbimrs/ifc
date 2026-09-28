# Changelog -- ifc-alignment

All notable changes to the `ifc-alignment` crate are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate follows Semantic Versioning independently of its siblings:
a release here does not imply a release of any other crate in the family.

Changes up to and including 0.2.0 are recorded family-wide in the
[repository changelog](../../CHANGELOG.md); that file is the archive for
everything released before per-crate changelogs began.

## [Unreleased]

### Changed (breaking)

- The read-side `HorizontalSegment`, `CantSegment`, `CantLayout`,
  `CantAtStation`, `LinearPlacement`, `StationEquation`,
  `LoweredAlignmentCurve`, `PartialHorizontalLayout` and `RefusedSegment`
  are `#[non_exhaustive]`; they can no longer be built with a struct literal
  outside the crate.

### Changed

- Depends on `ifc-schema` with its default features named explicitly
  (every bundled release), now that the workspace dependency turns them
  off for the facade's per-release features (#112).
- Pinned by test: `AlignmentView::for_model` refuses `IFC4X1` and
  `IFC4X2` (their alignment model differs from IFC4X3).

## [0.3.2] - 2026-09-28

### Fixed

- Referent authoring writes the SELECT values typed (#201):
  - `point_by_distance` writes `DistanceAlong` as `IFCLENGTHMEASURE(..)`.
    It is an `IfcCurveMeasureSelect`, where the wrapper is what tells a
    length from a curve parameter. The offsets stay bare.
  - `stationing` writes `Pset_Stationing`'s `NominalValue`s, declared
    `IfcValue`, as `IFCLENGTHMEASURE(..)` for `Station` and
    `IncomingStation` and `IFCBOOLEAN(..)` for `HasIncreasingStation`.

  `station_equations` reads both forms, as before.

## [0.3.1] - 2026-09-27

### Changed

- `profile_law` (and so `lower_gradient_curve`) now refuses a vertical
  profile whose seams do not join: a segment's `StartHeight` must match the
  previous segment's end height and its `StartGradient` the previous
  `EndGradient`, within the same magnitude-scaled tolerance already used for
  `StartDistAlong` contiguity. A height step or grade kink was previously
  accepted and silently shifted every downstream height. The refusal is the
  new `AlignmentError::ProfileDiscontinuity`, naming both segments, the
  discontinuous quantity (`ProfileSeam::Height` or `ProfileSeam::Gradient`)
  and both values (#95).

## [0.3.0] - 2026-09-23

### Changed

- **Breaking:** requires Axiolid 0.3. Lowered alignment curves are returned as
  `axiolid_model::GeometryGraph` and `NodeId`, so the major Axiolid version is
  part of this crate's public API. No code change.

## [0.2.0] - 2026-09-22

First release under per-crate versioning. See the
[repository changelog](../../CHANGELOG.md) for the family-wide history
that produced this version.

[Unreleased]: https://github.com/openbimrs/ifc/compare/ifc-alignment-v0.3.2...HEAD
[0.3.2]: https://github.com/openbimrs/ifc/releases/tag/ifc-alignment-v0.3.2
[0.3.1]: https://github.com/openbimrs/ifc/releases/tag/ifc-alignment-v0.3.1
[0.3.0]: https://github.com/openbimrs/ifc/releases/tag/ifc-alignment-v0.3.0
[0.2.0]: https://github.com/openbimrs/ifc/releases/tag/v0.2.0
