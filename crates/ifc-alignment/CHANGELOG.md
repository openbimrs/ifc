# Changelog -- ifc-alignment

All notable changes to the `ifc-alignment` crate are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate follows Semantic Versioning independently of its siblings:
a release here does not imply a release of any other crate in the family.

Changes up to and including 0.2.0 are recorded family-wide in the
[repository changelog](../../CHANGELOG.md); that file is the archive for
everything released before per-crate changelogs began.

## [Unreleased]

A behaviour fix with no API change, but a patch release is not enough:
input that evaluated before is now refused (a Viennese bend whose
rotation point moves), so the next release is a minor one (0.7.0).

### Fixed

- `cant_at`, `CantLayout::cant_at_distance` and so
  `CantLayout::frame_at_distance` evaluate a `VIENNESEBEND` for the
  section's cant `D = left - right`, as IFC4.3 ADD2 writes it
  (`IfcAlignmentCantSegmentTypeEnum`, 8.7.2.1): `psi = arcsin(D / b)`,
  `psi(xi) = psi1 + dpsi xi^4 (35 - 84 xi + 70 xi^2 - 20 xi^3)`,
  `D = b sin(psi)`, with the rails `e +- D / 2` about a rotation point
  `e = (left + right) / 2` that stays put (#312). They no longer blend
  each rail's height as if it were a cant: rotating about the centreline
  from `D = 0` to `0.15 m` on `b = 1.5 m`, the cant mid-bend is now
  0.0750942 m, not 0.0750235 m. Inside a bend `CantFrame` now agrees with
  the banked centreline (`Curve3::Banked`) and with the horizontal
  Viennese-bend law, which already used `D`.

### Changed

- Inside a `VIENNESEBEND` whose rotation point moves (rotation about the
  low rail, say), cant evaluation now returns
  `AlignmentError::Unsupported`, the refusal and reason the banked
  lowering already gives (#364 tracks the held-rail case). The authored
  start and end values still evaluate, so the horizontal Viennese-bend
  law, which reads the cant at its own ends, is unaffected where those
  meet the cant bend's ends. `CantLayout`
  compares the two rotation points at the model's declared precision, as
  the banked lowering does, so `CantLayout::resolve` of a layout holding a
  Viennese bend now also refuses a malformed
  `IfcGeometricRepresentationContext.Precision`. `cant_at` alone compares
  them to floating-point rounding.
- A Viennese bend's `|D| > b` check applies to the cant `D`, not to each
  rail's height.

## [0.6.0] - 2026-10-03

Input this crate refused now lowers exactly, onto the Axiolid relations
of axiolid-curve 0.3.3 and axiolid-model 0.3.4 (kernel#238, #239, #240),
and `HorizontalPlan::curve` can now be a `Curve2::Chain`: behaviour
changes, so the next release is a minor one (0.6.0).

### Added

- `lower_segmented_reference_curve` lowers an alignment with cant to an
  exact `Curve3::Banked` instead of refusing it, and
  `segmented_reference_curve3` returns that curve directly (#93). The cant
  law is `D = left - right` and the pivot `(left + right) / 2`, one
  `CantPiece` per cant segment in the IFC4.3 base formula (the Helmert
  transition as its two halves, the Viennese bend as its bank angle);
  the section rolls about the 3D tangent by `arcsin(D / b)`
  (`BankConvention::TangentRotation`, the reading IFC4.3 ADD2 states).

### Changed

- A `CUBIC` horizontal segment lowers on every path (#90): per segment and
  in the composite as its exact cubic Bezier trimmed at
  `TrimSelector::ArcLength(SegmentLength)`, and in the plan as a
  `ChainPiece2::Parametric` read by arc length, which makes
  `lower_horizontal_plan` return a `Curve2::Chain` for a layout holding
  one. The seam after a `CUBIC` reports its position (and heading) as
  `SeamCheck::Authored`.
- A vertical `CIRCULARARC` lowers to `ElevationLaw::CircularArc` from
  `StartHeight`, `StartGradient` and the signed `RadiusOfCurvature`, on
  the per-segment path as the circle itself trimmed by angle (#258). A
  radius turning against the authored grades, or an arc turning vertical
  before its end, is `InvalidSegment`.
- A profile starting before the plan rebases a straddling circular arc
  exactly, as it already rebased a polynomial piece.

### Still refused, by name

- A vertical `CLOTHOID`: its segment states neither end curvature
  (`RadiusOfCurvature` is defined for arcs and parabolas only), so its
  law is undetermined.
- A `CUBIC` that starts curved: IFC4.3 defines only `y = x^3 / (6 R L)`
  leaving a straight.
- A pivot that moves through a Viennese bend (Axiolid's
  `BankError::AngleInPivot`), a cant layout that does not span the plan,
  and `|D| > b`.

## [0.5.0] - 2026-10-02

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
  refusing a gap or overlap (#238).
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
- `vertical_profile_law(model, vertical, units)`: the exact profile of an
  `IfcAlignmentVertical`, with its seams checked at the precision the file
  declares (#141).
- `SeamTolerance` and `profile_law_within`: the seam tolerance as a value.
  `SeamTolerance::for_model` reads `Precision` from the model's 3D
  `IfcGeometricRepresentationContext`s (the coarsest, converted from the
  project length unit, capped at 1 mm); `profile_law` keeps the
  rounding-only rule (#141).
- `CantLayout::for_alignment`: the alignment's sole cant layout; none or
  several are a `SemanticViolation` (#93).
- `CantLayout::frame_at_distance` and `CantFrame`: cant at a station as rail
  heights, cant `D = left - right`, bank angle `arcsin(D / b)`, the
  rotation-point elevation `(left + right) / 2` and the section frame;
  `CantFrame::orient` places it on a caller-evaluated point and tangent
  (#93).
- `lower_segmented_reference_curve`: the cant-carrying centreline
  (`IfcSegmentedReferenceCurve` role). It resolves the cant layout and then
  refuses with `Unsupported`, because the pinned neutral curve vocabulary
  has no roll law (#93).
- `VerticalLayout::seams` lists every seam between vertical segments of
  positive length as a `VerticalSeam` (previous and next segment, station,
  incoming and outgoing grade) with a `VerticalSeamKind`: `Tangential` or
  `GradeBreak`. `VerticalLayout::require_tangential` refuses the first
  grade break with `ProfileDiscontinuity { seam: ProfileSeam::Gradient }`
  for a consumer that needs a tangent profile (#259).
- `real_export_survey` (ignored test): runs every alignment path over a
  local directory of real IFC4X3 exports (`IFC_ALIGNMENT_EXPORTS`) and
  tallies acceptances and refusals by cause.

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
- `lower_vertical_segment` lowers through `elevation_law`, like the composed
  gradient curve, so both paths accept and refuse the same segments.
  `PARABOLICARC` now lowers exactly to a quadratic Bezier in the
  (distance along, height) plane parameterised by plan distance; it was
  refused (#91).
- Vertical `CIRCULARARC` and `CLOTHOID` stay a typed `Unsupported`
  refusal, now naming the family and why it has no polynomial law; exact
  lowering waits on an Axiolid elevation law (#91, #258).
- `lower_gradient_curve` and `gradient_curve3` check vertical seams at the
  model's declared precision instead of rounding only, so files whose
  exporter rounds `StartDistAlong` or `StartHeight` to that precision
  compose (buildingSMART's `BC003_*` alignment reference datasets did not)
  (#141).
- A `CONSTANTGRADIENT` segment may carry `RadiusOfCurvature = 0.` and start
  and end gradients equal up to rounding, as real exports write them; both
  were refused.
- `VerticalLayout::resolve` checks contiguity with the
  same `SeamTolerance` rule as `vertical_profile_law`, at the model's
  declared precision (#141).

- Every layout path accepts the zero-length segment IFC4.3 requires at the
  end of each horizontal, vertical and cant layout (concept template
  *Alignment Layout - Horizontal, Vertical and Cant*). It adds no geometry
  and no curve piece; its start is still checked against the end of the
  previous segment by that layout's seam rule (closed-form position and
  heading, station and height, station and cant), and the horizontal seam
  is reported last in `seams`. A zero-length segment anywhere else, or as
  a layout's only segment, is refused with `SemanticViolation`; the cant
  and vertical paths accepted one anywhere before, and the horizontal
  reader refused every one. Covers
  `lower_horizontal_plan`, `lower_horizontal_layout`,
  `lower_horizontal_layout_partial`, `profile_law`/`profile_law_within`,
  `vertical_profile_law`, `VerticalLayout::resolve`, `CantLayout::resolve`
  and the gradient curve (#262).
- `read_horizontal_segment` reads `SegmentLength = 0` (the schema type is
  `IfcNonNegativeLengthMeasure`); only a negative length is refused (#262).
- `lower_horizontal_segment` and `lower_vertical_segment` refuse a
  zero-length segment with `InvalidSegment`: on its own it has no geometry.
  A zero-length `CONSTANTGRADIENT` lowered to a degenerate line before
  (#262).
- A grade break at a vertical seam whose height is continuous is accepted
  by `profile_law`, `profile_law_within`, `vertical_profile_law`,
  `VerticalLayout::resolve` and the gradient curve. IFC4.3 ADD2 does not
  require vertical seams to be tangential
  (`IfcAlignmentVerticalSegment`), and real exports carry such breaks
  (buildingSMART `BC003_ALX2`, Trimble). The piecewise elevation law
  carries the kink exactly: each piece keeps its own grade. A height step
  beyond `SeamTolerance` is still refused, and `ProfileSeam::Gradient` is
  now produced only by `require_tangential` (#259).

### Deprecated

- `station_equations`, which merges the referents of every alignment in the
  model into one table. It keeps working; use `Stationing::resolve` (#240).

### Fixed

- A clockwise `CIRCULARARC` plan elevated through `lower_gradient_curve`
  ran backwards: the evaluator reads a circle plan's distance
  counter-clockwise, so a negative radius was traversed in reverse. The
  intrinsic plan carries the signed curvature instead.
- `lower_gradient_curve` and `gradient_curve3` read the vertical profile at
  its own stations. `StartDistAlong` is measured from the start of the
  horizontal layout, but the profile was read from plan distance 0, so a
  profile starting at station `s` put every height `s` metres early. A
  profile starting before the plan is now re-indexed exactly (Taylor shift
  of the straddling piece); one starting after the plan start, or ending
  before the plan ends (beyond the seam tolerance), is refused with
  `Unsupported`, because the composed curve has no domain to leave those
  stations without heights.

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

[Unreleased]: https://github.com/openbimrs/ifc/compare/ifc-alignment-v0.6.0...HEAD
[0.6.0]: https://github.com/openbimrs/ifc/releases/tag/ifc-alignment-v0.6.0
[0.5.0]: https://github.com/openbimrs/ifc/releases/tag/ifc-alignment-v0.5.0
[0.4.0]: https://github.com/openbimrs/ifc/releases/tag/ifc-alignment-v0.4.0
[0.3.2]: https://github.com/openbimrs/ifc/releases/tag/ifc-alignment-v0.3.2
[0.3.1]: https://github.com/openbimrs/ifc/releases/tag/ifc-alignment-v0.3.1
[0.3.0]: https://github.com/openbimrs/ifc/releases/tag/ifc-alignment-v0.3.0
[0.2.0]: https://github.com/openbimrs/ifc/releases/tag/v0.2.0
