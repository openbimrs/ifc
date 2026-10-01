# Changelog -- ifc-alignment

All notable changes to the `ifc-alignment` crate are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate follows Semantic Versioning independently of its siblings:
a release here does not imply a release of any other crate in the family.

Changes up to and including 0.2.0 are recorded family-wide in the
[repository changelog](../../CHANGELOG.md); that file is the archive for
everything released before per-crate changelogs began.

## [Unreleased]

### Added

- `lower_horizontal_plan` lowers a whole `IfcAlignmentHorizontal` to one
  exact `Curve2::Intrinsic`: the first segment's start frame and a
  `CurvatureLaw::Piecewise` with one piece per segment. `HorizontalPlan`
  carries it with its sources and seams (#92).
- `HorizontalSeam` and `SeamCheck` report every seam between horizontal
  segments and whether its position was verified in closed form or
  accepted as authored after a transition spiral.
  `LoweredAlignmentCurve::seams` carries them for layout and gradient
  lowerings (#239).

- `AlignmentView::hierarchy` returns an `AlignmentHierarchy`: the
  horizontal, vertical and cant layouts and the `IfcReferent`s an
  `IfcAlignment` nests (`IfcRelNests`, in nesting order), its parent and
  child alignments (`IfcRelAggregates`), and the products it positions
  (`IfcRelPositions`). `sole_horizontal`, `sole_vertical` and `sole_cant`
  refuse several layouts of a kind instead of picking one;
  `governing_horizontal` follows a child alignment to the horizontal
  layout its parent nests; `layout_segments` lists a layout's parameter
  segments; `alignments` and `model` round out the view (#238).
- `VerticalLayout::resolve`, the vertical counterpart of
  `CantLayout::resolve`: the ordered segments of an `IfcAlignmentVertical`,
  refusing a gap or overlap and a kink in grade (#238).
- `Stationing::resolve` scopes station equations to one alignment, from
  the referents it nests or positions, and checks each `IncomingStation`
  (or a plain continuation) against the station carried from the previous
  referent within `STATION_TOLERANCE`. `station_at`, `distance_at` and
  `distances_at` map between station and distance along across equations
  and decreasing stationing; a station that occurs twice is
  `AlignmentError::AmbiguousStation`, one outside the table
  `AlignmentError::OutOfRange` (#240).
- The IFC4X3 ADD2 declaration and slot inventory is pinned by tests against
  the bundled schema table: every slot and arity the readers and authoring
  index, the declarations read, the subtype and SELECT memberships relied
  on, every segment `PredefinedType` member, and the absence of the layouts
  from IFC2X3 through IFC4X2 (#16).

### Changed

- `lower_gradient_curve` and `gradient_curve3` elevate a multi-segment plan
  instead of refusing it, using `lower_horizontal_plan`. The plan is now
  always that intrinsic curve, also for a single segment, where it used to
  be the segment's basis line, circle or spiral. A heading kink between
  segments is refused, since one intrinsic curve cannot carry it (#92).
- `lower_horizontal_layout` lowers a layout containing transition spirals
  to one composite instead of refusing it. A seam after a spiral is
  accepted as authored, reported in `seams`, and declares
  `Transition::Discontinuous` (no claim); a gap after a line or arc is
  still refused. `lower_horizontal_layout_partial` no longer splits runs
  at spirals, so `PartialHorizontalLayout::is_complete` again means one
  run (#239).
- A `CUBIC` horizontal segment is refused with its own reason: its end on
  the curve inverts a non-elementary arc-length integral, and the pinned
  neutral vocabulary has no arc-length trim. A `CUBIC` with a non-positive
  length, non-finite radii or equal start and end radii is reported as
  `InvalidSegment` first (#90, still open).

### Deprecated

- `station_equations`, which merges the referents of every alignment in the
  model into one table. It keeps working; use `Stationing::resolve` (#240).

### Fixed

- A clockwise `CIRCULARARC` plan elevated through `lower_gradient_curve`
  ran backwards: the evaluator reads a circle plan's distance
  counter-clockwise, so a negative radius was traversed in reverse. The
  intrinsic plan carries the signed curvature instead.


## [0.4.0] - 2026-09-29

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

[Unreleased]: https://github.com/openbimrs/ifc/compare/ifc-alignment-v0.4.0...HEAD
[0.4.0]: https://github.com/openbimrs/ifc/releases/tag/ifc-alignment-v0.4.0
[0.3.2]: https://github.com/openbimrs/ifc/releases/tag/ifc-alignment-v0.3.2
[0.3.1]: https://github.com/openbimrs/ifc/releases/tag/ifc-alignment-v0.3.1
[0.3.0]: https://github.com/openbimrs/ifc/releases/tag/ifc-alignment-v0.3.0
[0.2.0]: https://github.com/openbimrs/ifc/releases/tag/v0.2.0
