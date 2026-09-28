# Changelog -- ifc-alignment

All notable changes to the `ifc-alignment` crate are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate follows Semantic Versioning independently of its siblings:
a release here does not imply a release of any other crate in the family.

Changes up to and including 0.2.0 are recorded family-wide in the
[repository changelog](../../CHANGELOG.md); that file is the archive for
everything released before per-crate changelogs began.

## [Unreleased]

### Fixed

- `read_vertical_segment`, `read_cant_segment` and `CantLayout::resolve`
  (`RailHeadDistance`) read a typed parameter such as
  `IFCLENGTHMEASURE(1.)` exactly like the bare number, as
  `read_horizontal_segment` already did; they refused it as the wrong kind
  (#140).

### Changed (breaking)

- The authoring drafts `HorizontalSegmentDraft`, `VerticalSegmentDraft`
  and `CantSegmentDraft` are `#[non_exhaustive]`: build them with
  `HorizontalSegmentDraft::new(start_point, start_direction, start_radius,
  end_radius, segment_length, predefined_type)`,
  `VerticalSegmentDraft::new(start_dist_along, horizontal_length,
  start_height, start_gradient, end_gradient, predefined_type)` or
  `CantSegmentDraft::new(start_dist_along, horizontal_length,
  start_cant_left, start_cant_right, predefined_type)` and the setters
  `gravity_center_line_height`, `radius_of_curvature`, `end_cant_left` and
  `end_cant_right`. Fields stay public.
- The read-side `HorizontalSegment`, `CantSegment`, `CantLayout`,
  `CantAtStation`, `LinearPlacement`, `StationEquation`,
  `LoweredAlignmentCurve`, `PartialHorizontalLayout` and `RefusedSegment`
  are `#[non_exhaustive]`; they can no longer be built with a struct literal
  outside the crate.

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
