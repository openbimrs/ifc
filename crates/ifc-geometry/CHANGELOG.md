# Changelog -- ifc-geometry

All notable changes to the `ifc-geometry` crate are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate follows Semantic Versioning independently of its siblings:
a release here does not imply a release of any other crate in the family.

Changes up to and including 0.2.0 are recorded family-wide in the
[repository changelog](../../CHANGELOG.md); that file is the archive for
everything released before per-crate changelogs began.

## [Unreleased]

## [0.21.0] - 2026-10-10

### Added (#367, the neutral graph as a value)

- Feature `wire` (off by default): `ifc_geometry::wire`, a re-export of
  `axiolid_model::wire` (Axiolid's versioned wire format of a
  `GeometryGraph`, Axiolid ADR 0085: `FORMAT_NAME`, `FORMAT_VERSION` 1.0,
  `FormatVersion`, `WireError`), and with it `GeometryGraph::to_json`,
  `from_json`, `to_cbor` and `from_cbor`, through `axiolid-model`'s
  `serde` feature. It encodes the graph lowering produces and computes
  nothing (ADR 0004).

### Changed

- `axiolid-model` floor 0.3.8 (axiolid/kernel#267), which brings
  `axiolid-core` 0.3.2, `axiolid-curve` 0.3.7, `axiolid-mesh` 0.3.3,
  `axiolid-primitive` 0.3.3, `axiolid-profile` 0.3.2, `axiolid-surface`
  0.3.3, `axiolid-topology` 0.3.2 and `axiolid-linear` 0.3.2; all
  additive.

Semver: additive (a new opt-in feature), a minor release while 0.x, since
the dependency floors rise.

## [0.20.0] - 2026-10-10

Semver: a minor release. Behaviour changes: a linear placement whose basis
curve is a curve relation, refused by name until now, derives through a
`CurveEvaluator`, and an evaluator without curve paths is refused there by
a new `GeometryError` variant (the enum is `#[non_exhaustive]`); stations
and derived placements along offset curves, refused until now, lower and
derive; an `IfcOffsetCurve2D` lowers with a reference direction where it
carried none, a change in the lowered graph. A public function is added.
The dependency floors rise with the workspace.

### Added

- A derived `IfcLinearPlacement` on a curve-relation basis (#418): a plain
  `IfcCompositeCurve`, an `IfcTrimmedCurve`, or a composite of
  `IfcCurveSegment`s placed by `IfcAxis2PlacementLinear`s. The basis is
  read as Axiolid's neutral `CurvePath` (axiolid/kernel#290), built from
  the stored relation the station lowering reads (#346), each segment's
  station framed through the caller's evaluator; the frame comes from
  `CurveEvaluator::path_frame_at` off a joint and
  `path_frame_at_on(.., SeamSide::Incoming)` within precision of one, as
  #409 reads a seam. It equals the lowered station's frame on and off
  joints, and `CachedPositionPolicy::Verify` accepts a cache computed with
  the incoming segment and refuses one computed with the outgoing segment.
  An `IfcParameterValue` along such a basis and a composite whose pieces
  do not meet are refused by name.
- `constraint::placement::derive::basis_curve_path` (`compile` feature):
  the `CurvePath` a derivation reads along a curve-relation basis. A test
  pins it equal to the path `axiolid-mesh-compile`'s
  `station::curve_path` builds from the lowered graph.
- `GeometryError::CurvePathUnsupported` (`compile` feature) names the
  placement and its basis curve when a caller's evaluator does not
  implement curve paths and refuses them with `CURVE_PATH_UNSUPPORTED`. A
  frame read on one piece is never used in its place. `is_unsupported()` is
  true for it.
- Stations on offset-curve bases (#414): an `IfcPointByDistanceExpression`,
  an `IfcAxis2PlacementLinear`, and runs of sections and offsets whose
  basis curve is an `IfcOffsetCurve2D`, an `IfcOffsetCurve3D` or an
  `IfcOffsetCurveByDistances`, nested in composites and placements too,
  lower onto Axiolid's offset station basis (axiolid/kernel#289, Axiolid
  ADR 0082 amendment 2026-10-10). A distance runs along the offset's OWN
  length, not its basis's. The offset's joints, one per seam of its basis
  (every polyline vertex, every joint of a composite) and per station of an
  offset by distances, are read from the stored relation as the compiler
  flattens it (`lower::station::relation::offset`): beside a line or a
  polyline edge a line, constant or linear between stations, of length
  `sqrt((L + dg)^2 + dl^2 + dv^2)`; beside a circle, constantly in its
  plane, a circle of radius `r - a`, `(r - a) / r` times the arc's length.
  A station within the model's precision of a joint stands on it reading
  `SeamSide::Incoming`, the previous segment (IFC4.3 ADD2 8.9.3.48.3). The
  joints and lengths are pinned to the kernel's own within
  `ARC_LENGTH_TOLERANCE`: `station::seams` where the kernel states them
  exactly, the composite's seams along `station::curve_path` where it
  measures a polyline offset by quadrature. A derived `IfcLinearPlacement`
  on an offset basis reads it as a `CurvePath` of `PathCurve::Offset`
  pieces (#418's path builder), equal to `station::curve_path`'s and to the
  lowered station's frame on the kerb's corner.
- `synthetic-lowering/station_offsets_ifc4x3.ifc`: an `IfcOffsetCurve2D` of
  a line and a tangent quarter circle (a joint whose sides meet, an arc of
  radius 9), of a quarter circle (radius 11, 5.5 pi m), an
  `IfcOffsetCurveByDistances` of an L polyline vanishing at its corner, a
  section run and an offset run along offsets, and an `IfcOffsetCurve2D`
  across the L's corner; tests through the reference kernel on and 4 um
  past each joint.

### Changed

- Requires `axiolid-curve` 0.3.6, `axiolid-curve-evaluate-contract` 0.3.4
  and, behind `compile`, `axiolid-mesh-compile` 0.3.19 (and
  `axiolid-evaluate` 0.3.10 for the tests): the releases that add curve
  paths (axiolid/kernel#290) and measure a station along an offset
  (axiolid/kernel#289). The lock also moves `axiolid-nurbs` 0.3.6,
  `axiolid-brep-boolean` 0.1.9 and `axiolid-overlay` 0.3.12. Every other
  test passes unchanged against them.
- A closed conic trimmed across its parameter seam inside a relation basis
  is located from its start parameter taken modulo one turn, as Axiolid
  locates it, so its span starts within the curve's measure.
- The refusal of a basis curve the derivation does not read names the
  composite and trimmed curves among those it does.
- An `IfcOffsetCurve2D` lowers to `CurveRelation::Offset` with its frame's
  `+Z` as `reference_direction`, where it carried `None` (#414). Every curve
  lowers to a 3D curve, a 2D one in `z = 0` of its frame, and IFC's
  positive `Distance` "in the sense of an anti-clockwise rotation through
  90 degrees from the tangent vector T" (8.9.3.40) is `Z x T` there, the
  3D offset's `V x T` (8.9.3.41) and Axiolid's left lateral: the same
  curve. Axiolid refuses a planar offset law on a 3D curve as a station
  basis, so `None` left nothing measurable along it. Nothing meshed an
  offset relation outside stations, so no compiled result changes.
- The refusals of offset bases now name what the kernel refuses: an
  offset of an offset, a trim of an offset curve (it takes its basis's
  parameterisation, not its own length), an offset through a circle's
  centre, and an offset whose length is a quadrature (beside an ellipse, a
  B-spline, a spiral or a gradient curve, or a variable offset of a
  circle). An offset across a corner of its basis, whose sides do not meet
  (IFC: the basis "shall have a well-defined tangent direction at every
  point"), and a 3D offset whose tangent runs along its reference
  direction lower, and the kernel refuses them by name, typed. The
  dispatch ledger's `IfcPointByDistanceExpression` rows say so.

## [0.19.0] - 2026-10-09

Semver: a minor release. Behaviour changes on seams: a linear placement
derived through a `CurveEvaluator` on a tangent discontinuity takes the
incoming tangent, and an evaluator that cannot read that side is now
refused there. `GeometryError` gains a variant (the enum is
`#[non_exhaustive]`). The dependency floors rise with the workspace
(#364, below); the seam sides are axiolid/kernel#286 in the same releases.

### Changed

- Requires `axiolid-curve` 0.3.5, `axiolid-curve-evaluate-contract` 0.3.3
  and, behind `compile`, `axiolid-mesh-compile` 0.3.17 (and
  `axiolid-evaluate` 0.3.9 for the tests): the releases that rotate a
  banked curve about a held rail (axiolid/kernel#279), which
  `ifc-alignment` now lowers a Viennese bend onto. Nothing here builds a
  banked curve; every test passes unchanged against them.
- A derived `IfcLinearPlacement` on a tangent discontinuity takes the
  incoming tangent (#409), as IFC4.3 ADD2 8.9.3.48.3 asks ("the tangent of
  the previous segment governs") and as the lowered station has since
  #346. `derive_placement_transform` locates the seams as `lower::station`
  does, from the basis curve's stored data, snaps a `DistanceAlong` within
  the model's precision (capped at 1 mm) to the seam's distance, and asks
  the evaluator for `CurveEvaluator::frame_at_on(.., SeamSide::Incoming)`
  there (axiolid/kernel#286). A polyline parameter on a vertex is read at
  the vertex's distance. Off a seam nothing changes. Before, the derived
  frame on a grade break or a polyline corner was the outgoing one, so it
  disagreed with the same station lowered as geometry, and
  `CachedPositionPolicy::Verify` refused a cache an exporter computed with
  the incoming tangent while accepting one computed with the outgoing
  tangent; now the reverse holds.

### Added

- `GeometryError::SeamSideUnsupported` (`compile` feature) names the
  placement, its basis curve and the seam's distance when a caller's
  evaluator does not implement seam sides and refuses `Incoming` with
  `SEAM_SIDE_UNSUPPORTED`. The outgoing frame is never used in its place.
  `is_unsupported()` is true for it.

### Documentation

- The derived frame's documentation states the evaluator's axes as
  kernel#242 documents them (x tangent, y up, z right); the derivation
  reads only the tangent, and a test pins the IFC frame
  `(tangent, left, up)` to `(x, -z, y)` on and off a seam.

## [0.18.0] - 2026-10-09

Semver: a minor release. No public item is added, removed or changes
signature, but behaviour changes: stations along curve relations (#346)
and `IfcCurveSegment`s placed by an `IfcAxis2PlacementLinear` (#311),
refused by name until now, lower. The Axiolid requirements are the ones
#398 raised (`axiolid-model` 0.3.7, `axiolid-mesh-compile` 0.3.16).

### Added

- Stations along curve relations (#346). Axiolid measures a station along
  a composite, a trim, a surface curve whose 3D curve governs and a curve
  placed at a station since `axiolid-model` 0.3.7 (axiolid/kernel#285, ADR
  0082 amendment): end to end through the pieces, each in its own curve's
  convention, every joint a seam. So an `IfcPointByDistanceExpression`,
  an `IfcAxis2PlacementLinear` and the runs of `IfcSectionedSolidHorizontal`,
  `IfcSectionedSurface` and `IfcOffsetCurveByDistances` on a plain
  `IfcCompositeCurve`, an `IfcTrimmedCurve` or a composite of segments
  placed at stations lower. On a joint, and within the model's precision
  of one, a station stands on the joint reading `SeamSide::Incoming`, the
  previous segment, as IFC4.3 ADD2 8.9.3.48.3 asks. The joints are read
  from the stored relation (`lower::station::relation`, repeating the
  compiler's flattening, since `axiolid-evaluate` is an execution provider
  this crate does not link): pieces' lengths from a line's direction, a
  circle's radius, a polyline's edges and the measure of a curve
  parameterised by it; trims by parameter, by arc length, or by a point on
  a line or a circle, a closed conic's within one turn; a segment whose
  sense disagrees reversed. The tests pin them against the kernel's own
  seams.
- An `IfcCurveSegment` placed by an `IfcAxis2PlacementLinear` (#311)
  lowers to an `InstanceAtStation`: the parent's piece in local
  coordinates (start at the origin, tangent along `x`) placed in the frame
  of the `OrientedCurveStation` the placement lowers to, whose local `x`,
  `y`, `z` are the station's tangent, left and up as IFC4.3 ADD2 8.9.3.4
  reads them. A composite of such segments is measured in the segments'
  own arc length, whatever base they stand on. Part of #311:
  `IfcSegmentedReferenceCurve` stays refused (below).
- `synthetic-lowering/station_relations_ifc4x3.ifc`: a plain composite of
  a polyline, a polyline stored backwards (`SameSense` false) and a
  quarter circle trimmed by Cartesian points, with points on its joints
  and 4 um past one; a composite with a gap; and two `IfcLine` segments placed at stations 10 and 30 of
  a gradient curve. Tests resolve them through the reference kernel
  against closed forms.

### Changed

- `IfcSegmentedReferenceCurve`'s refusal (`dispatch::PLANNED`) names what
  remains open in #311 precisely: its segments are curves in the
  (distance along, deviating elevation) space of its base curve, and no
  normative rule maps a segment's `ParentCurve` and placement `Axis` to a
  cant law and pivot; implementations differ in the coefficient scaling
  and the cross-slope formula.
- An `IfcCurveSegment` in an `IfcGradientCurve` placed by an
  `IfcAxis2PlacementLinear` keeps its refusal, now naming why: a gradient
  curve's segments are read as plan and profile pieces, which need a
  resolved placement.
- Uses the Axiolid releases #398 raised the workspace to (`axiolid-model`
  0.3.7, axiolid/kernel#285 for stations along relations). Upstream now
  refuses a tilted placement of a plan-measured curve only, reading a
  tilted arc-length one in its own reference-up frame; nothing lowered
  before reached that rule, and no test's result changed with the bump
  alone.

### Refused by name

- A station along an offset curve (`IfcOffsetCurve2D`/`3D`), which
  Axiolid measures no station along; along a relation joining a gradient
  curve with arc-length pieces, through which no one distance runs; and
  along a relation with an ellipse or a B-spline piece, or a trim whose
  ends are not stated by its data, whose joints lie at an arc-length
  integral. A composite whose pieces do not meet, or whose undeclared
  sense runs a piece backwards, is refused by the kernel.

## [0.17.0] - 2026-10-09

Semver: a minor release. No public item is added, removed or changes
signature, but behaviour changes as it did for #393: half-space boundaries
with circular arcs, refused as `Unsupported` until now, lower, so a
consumer matching on the boundary node now meets a `Profile` as well as a
`Curve2`; `PARTIAL` loses the family's two rows; and the Axiolid
requirements rise (`axiolid-model` 0.3.7, `axiolid-evaluate` 0.3.8, and
with `compile-reference-backend` `axiolid-mesh-compile` 0.3.16 and
`axiolid-construct` 0.3.17).

### Added

- `IfcPolygonalBoundedHalfSpace` boundaries with circular-arc segments
  (#398): an `IfcCompositeCurve` with trimmed `IfcCircle` segments, or an
  `IfcIndexedPolyCurve` with a non-collinear `IfcArcIndex`, lowers to a
  `Profile::Contour` boundary (no holes) whose arcs are exact `Circle2`
  segments, never chords. `SolidOperation::BoundedHalfSpace` takes a
  profile boundary since `axiolid-model` 0.3.7 (axiolid/kernel#277,
  Axiolid ADR 0084): the reference exact compiler clips by a right
  circular cylinder per arc, and the mesh compiler flattens the arcs
  under its chord budget with a certified deviation. A boundary of
  straight edges still lowers to the same closed `Polyline2` as before,
  so its output is unchanged. Still refused, by name: as `Degenerate`, a
  gap at an arc's end (judged within the model's `Precision`), an open
  curve, a point off the plane and an `IfcArcIndex` with coincident
  points; as `Unsupported`, a boundary circle placed by an
  `IfcAxis2Placement3D`, an `IfcCurveSegment` member, a reparametrised
  segment and any other curve family. A boundary that crosses or touches
  itself is the kernel's to refuse (`InvalidInput`, named alike in both
  compilers); it reaches the caller as `CompilationRefused`.
  `IfcPolygonalBoundedHalfSpace` is now listed as implemented, with no
  variant rows. `halfspace_boundaries_ifc4x3.ifc` gains two walls, one
  clipped by a six-segment composite with two arcs of radius 1.2, one by
  an indexed boundary with an `IfcArcIndex`; each compiles, exact and
  meshed, to its closed-form volume.

## [0.16.0] - 2026-10-09

Semver: a minor release (0.16.0). No public item is added, removed or
changes signature, but behaviour changes (#346): stations on a tangent
discontinuity and runs of sections and offsets across one, refused by
name until now, lower (an `IfcPointByDistanceExpression` on a seam to an
`OrientedCurveStation`, the only station that carries a seam side, where
one off a seam stays a `CurveStation`); and the Axiolid
requirements rise (`axiolid-model` 0.3.6, `axiolid-curve` 0.3.4, and with
`compile-reference-backend` `axiolid-mesh-compile` 0.3.15,
`axiolid-mesh-compile-contract` 0.3.3 and `axiolid-construct` 0.3.16). The
example below and the `compile` module's new section alone would be a
patch.

### Added

- `examples/backend_compare/` (#31; `required-features = ["compile"]`)
  compiles a fixed corpus of committed fixtures through every backend in
  one list, with no `cfg` in the traversal. The list holds the reference
  backend under `compile-reference-backend`, and `polygon-extruder`, a
  `MeshCompiler` defined in the example that names only the published
  contracts and builds under `compile` alone. `polygon-extruder` compiles
  collections and instances, triangle and polygon meshes, faceted B-reps,
  extrusions of polygonal profiles with holes, and blocks, and refuses
  everything else with `GeomError::UnsupportedInput`. The example prints:
  - wall time per fixture, keyed by `BackendId`: the first pass reported
    on its own, then the median and range of `--iterations` passes;
  - mesh size (vertices, triangles) and coverage (meshed, refused, no
    body) per fixture;
  - agreement on signed volume, area and bounding box within a stated
    relative tolerance, every divergence named by fixture, product, metric
    and both values;
  - every product one backend meshed and the other refused.

  The method, machine, profile and load are printed beside the numbers,
  which are not a cross-kernel performance claim. `scripts/gate.sh` runs it
  quick under both features and with `--strict` beside the reference. The
  `compile` module docs and ADR 0012 (amendment) link it.

- Stations on tangent discontinuities (#346). IFC4.3 ADD2 8.9.3.48.3:
  "If DistanceAlong coincides with a point of tangential discontinuity
  (within precision limits), then the tangent of the previous segment
  governs." Axiolid now states a seam side (`SeamSide`, `axiolid-model`
  0.3.6, ADR 0082 amendment, axiolid/kernel#263), so such a station lowers
  with `SeamSide::Incoming`: an `IfcAxis2PlacementLinear` as an
  `OrientedCurveStation` reading the incoming side, an
  `IfcPointByDistanceExpression` as `CurveStation::with_seam_side`'s
  unturned oriented station. `OffsetLongitudinal` then runs along the
  incoming tangent.
- Runs across tangent discontinuities (#346): `IfcSectionedSolidHorizontal`,
  `IfcSectionedSurface` and `IfcOffsetCurveByDistances` whose stations
  span a polyline corner or a grade break lower, and Axiolid cuts them in
  the half-angle mitre plane 8.8.3.35.1 and 8.8.3.37.1 ask for (the plane
  normal to the bisector of the two tangents). IFC states no join for the
  offset curve; the mitre is the one the sweeps on the same stations take.
- `synthetic-lowering/station_seams_ifc4x3.ifc`: a gradient curve with a
  grade break and an L-shaped polyline, with points on and within the
  precision of each seam, a sectioned solid and an offset curve (and a disk
  swept along it) across the corner, and a sectioned surface across the
  grade break. Tests resolve them through `axiolid-reference` and the
  reference mesh compiler against closed forms: the incoming tangent, the
  mitre's corners, the volume and the area.

### Changed

- A station within the model's declared `Precision` of a seam (capped at
  1 mm, or `1e-9 * max(1, |s|)` where larger) is stored at the seam's own
  distance (#346). Axiolid reads a station on a seam only within
  `ARC_LENGTH_TOLERANCE * max(1, s)` (`1e-12`); snapping to the distance
  Axiolid itself reads from the same stored data (pinned against
  `exact_station_seams` in the tests) makes the two rules agree, where a
  station 4 um off a corner would otherwise be on the corner for IFC and
  on one leg for the kernel. A station of a run is snapped the same way,
  so its section stands in the mitre plane; two that snap onto one seam
  are refused as out of order.
- The seam reader carries whether the curve turns back on itself at a
  seam, by Axiolid's `MITRE_TOLERANCE` (1e-6 on the cosine of half the
  turn), repeated in `lower::station::seams` because `axiolid-evaluate` is
  an execution provider this crate does not link (ADR 0004).
- Workspace requirements: `axiolid-model` 0.3.6, `axiolid-curve` 0.3.4,
  `axiolid-evaluate` 0.3.7 (dev), `axiolid-mesh-compile` 0.3.15,
  `axiolid-mesh-compile-contract` 0.3.3 and `axiolid-construct` 0.3.16.
  The kernel now reads a station within `ARC_LENGTH_TOLERANCE` of a seam
  at the seam; no test's result changed with the bump.

### Refused by name

- A run of sections or offsets across a seam where the basis turns back
  on itself: "very sharp edges may result in nearly impossible miter"
  (8.8.3.35.1), and the half-angle plane has no extent there.
- Still: a station on a basis whose seams cannot be read from stored data.
  A curve relation (a plain `IfcCompositeCurve`, a trimmed or offset
  curve): Axiolid resolves no station along one, which remains open in
  #346. A B-spline with a corner knot, whose distance is an arc-length
  integral (Axiolid's `exact_station_seams` refuses it too).

## [0.15.0] - 2026-10-09

Semver: an `IfcIndexedPolyCurve` whose collinear `IfcArcIndex` was
refused as `Degenerate` now lowers (#396), and `rules::validate` no longer
reports `BoundaryType` for an IFC4X3 `IfcIndexedPolyCurve` boundary or a
subtype of `IfcCompositeCurve` (#397). Every where-rule is now checked in
the file's declared release and reported under that release's name (#400):
an IFC2X3 file's violations are named `WR1`, `WR31`, ... as IFC2X3 TC1
names them, and the set of violations changes for IFC2X3 and IFC4X3 files
as listed below. No public item is added, removed or changes signature
(`rules::validate`, `rules::validate_model`, `rules::placement::check`,
`rules::solid::check` and `RuleViolation` are as before); a consumer
matching IFC2X3 violations by their IFC4 rule name must match the `WRnn`
name instead. Behaviour changes only, so the next release is a minor one
(0.15.0); nothing is breaking at the API level. No committed fixture's
violation set changes. The where-rules of the geometry entities IFC4 does
not declare (#402) add violations only, on files that break them, and
public API is unchanged, so they join the same minor release.

### Added

- `rules::validate` and `rules::validate_model` check the where-rules of
  the geometry entities IFC4 ADD2 TC1 does not declare, each in the
  releases that declare it, under that release's name and text (#402), and
  the `DECLARED` table lists them so the schema-backed tests hold them to
  each bundled release:
  - IFC2X3 TC1 `Ifc2DCompositeCurve.WR1` (`ClosedCurve`) and `WR2`
    (`Dim = 2`); `IfcRationalBezierCurve.WR1` (one weight per control
    point) and `WR2` (`IfcCurveWeightsPositive`);
  - IFC4X1 on: `IfcSectionedSolid.ConsistentProfileTypes`,
    `DirectrixIs3D` and `SectionsSameType`;
    `IfcSectionedSolidHorizontal.CorrespondingSectionPositions` and
    `NoLongitudinalOffsets` (IFC4X1 and IFC4X2 read the offset on the
    `IfcDistanceExpression` position, IFC4X3 ADD2 on the position's
    `IfcPointByDistanceExpression` location);
    `IfcTriangulatedIrregularNetwork.NotClosed` (a written `Closed = TRUE`
    violates; an omitted one is UNKNOWN and conforms);
  - IFC4X3 ADD2: `IfcAxis2PlacementLinear.WR1` (the location is an
    `IfcPointByDistanceExpression`) and `WR2` (Axis not parallel to
    RefDirection, read as `AxisToRefDirPosition` is);
    `IfcPolynomialCurve.CorrectPositionDim` and `ValidCoefficients`;
    `IfcSectionedSurface.AreaProfileTypes` (at least one `CURVE`
    cross-section, as its text demands), `CorrespondingSectionPositions`,
    `DirectrixIs3D`, `NoOffsets` and `SectionsSameType`.

  #402 listed the sectioned solids and the TIN as IFC4X3-only; IFC4X1 and
  IFC4X2 declare them too, and are checked. `IfcCurveWeightsPositive` is
  transcribed once (`rules/express.rs`) for both the Bezier curve and IFC4's
  `IfcRationalBSplineCurveWithKnots.WeightsGreaterZero`: its `Weights` array
  is `IfcListToArray(...) = ?` when `WeightsData` and `ControlPointsList`
  differ in length, and the function then returns TRUE, so a rational
  B-spline with mismatched lists now reports only
  `SameNumOfWeightsAndPoints`, not also `WeightsGreaterZero`. `Dim` follows
  `IfcPointDim` (IFC4X3 ADD2) and the `Dim` of `IfcPointOnCurve` and
  `IfcPointOnSurface` (every release): a point's `BasisCurve` or
  `BasisSurface`, and an `IfcPointByDistanceExpression`'s `BasisCurve`,
  where it was undecided.

### Changed

- The general curve lowering (`lower_curve_node`, and every representation
  item, axis or directrix that reaches it) treats a collinear `IfcArcIndex`
  as IFC4 ADD2 TC1 and IFC4X3 ADD2 prescribe: "The three points shall not
  be co-linear. In case that this informal proposition is not maintained,
  the arc segment shall be treated as a polyline segment" (#396). Three
  distinct points collinear within the model's `Precision` lower as a
  straight `Curve3::Polyline` segment of the composite, start -> end when
  the middle point lies between the others and start -> mid -> end when it
  does not; they were refused as `Degenerate`. Two points that coincide
  within `Precision` are still `Degenerate`. Both tests now use the model's
  `Precision` (in the curve's own coordinates, `1.E-5` project units when
  none is declared) instead of an absolute `f64::EPSILON` bound on a
  squared area, and they are one helper, `constraint::tolerance::arc_points`,
  shared with the profile and half-space boundary reader of #335 and #393,
  whose results are unchanged except that its tolerance, like every point
  comparison's, is floored at floating-point rounding for very large
  coordinates. A model whose declared `Precision` is not a finite positive
  number now refuses an indexed curve with an arc segment, as it refuses an
  indexed profile boundary. The fixture `indexed_curve_arcs.ifc` (IFC4)
  lowers representation curves with a collinear, an out-and-back collinear,
  a nearly collinear and a genuine arc, and `halfspace_boundaries_ifc4x3.ifc`
  gains a wall whose boundary has a collinear arc; both pass the dispatch
  corpus.

### Fixed

- The where-rule `IfcPolygonalBoundedHalfSpace.BoundaryType` is checked in
  the file's own release (#397). IFC4X3 ADD2 admits `IfcPolyline`,
  `IfcCompositeCurve` and `IfcIndexedPolyCurve`; IFC2X3 TC1 (`WR42`), IFC4
  ADD2 TC1, IFC4X1 and IFC4X2 admit only the first two. An IFC4X3 file with
  an indexed boundary was flagged `WrongType`. The release is the header's
  `FILE_SCHEMA`, IFC4 ADD2 TC1 when none known is declared, and, as
  `TYPEOF` includes supertypes, a subtype of an admitted type (an
  `IfcBoundaryCurve`) satisfies the rule, judged in that release's entity
  table.
- Every where-rule in `rules` is checked against the declared release's own
  text (#400). One helper, `rules/release.rs`, reads the release from
  `FILE_SCHEMA` (IFC4 ADD2 TC1 when none known is declared), lists which
  rules each bundled release declares on which entity, and answers every
  `IN TYPEOF` test in that release's entity table; no rule matches names by
  hand any more. Two tests hold it to the schemas: the rule list must equal
  each bundled release's own table, and each rule's EXPRESS text in every
  release (from `references/ifc-spec`) must be IFC4's except where the code
  branches on and cites the difference.
  - `IfcBooleanClippingResult.FirstOperandType`: IFC2X3 TC1 (`WR1`) admits
    only an `IfcSweptAreaSolid` or `IfcBooleanClippingResult`, so an IFC2X3
    swept disk is now reported. IFC4 ADD2 TC1 to IFC4X3 ADD2 spell the third
    disjunct `IFCSWEPTDISCSOLID`, which names no entity; it is read as
    `IfcSweptDiskSolid`, as buildingSMART/IFC4.x-development#927 and its
    open correction #1107 state, so a clipped swept disk stays admitted.
  - Rules IFC2X3 TC1 does not declare are no longer applied to IFC2X3
    files: `IfcBooleanResult.FirstOperandClosed`/`SecondOperandClosed`,
    `IfcDirection.MagnitudeGreaterZero`,
    `IfcRepresentationMap.ApplicableMappedRepr`, and `DirectrixBounded`.
  - `IfcRevolvedAreaSolid.AxisStartInXY` in IFC4X3 ADD2 also requires
    `Axis.Location` to be an `IfcCartesianPoint`; IFC4X3's `LocationIsCP`
    on `IfcAxis1Placement`, `IfcAxis2Placement2D` and `IfcAxis2Placement3D`
    is now checked too.
  - `DirectrixBounded` in IFC4X3 ADD2 is declared on
    `IfcDirectrixCurveSweptAreaSolid` and so binds every subtype, including
    `IfcDirectrixDerivedReferenceSweptAreaSolid`, which was not checked.
  - IFC2X3 TC1's `IfcSweptSurface.WR1` (no `IfcDerivedProfileDef` as the
    swept curve), dropped in IFC4, is checked for IFC2X3 files.
  - A violation names its rule as the declared release does (`WR1`-`WR5`
    on an IFC2X3 `IfcAxis2Placement3D`), and its `type_name` is the
    violating entity's own type (an `IfcExtrudedAreaSolidTapered` was
    reported as `IFCEXTRUDEDAREASOLID`).
  - Rules declared on a supertype now bind subtypes the name matching
    missed: `First`/`SecondOperandClosed` read `Closed` on an IFC4X1-on
    `IfcTriangulatedIrregularNetwork`, and `ApplicableMappedRepr` judges
    the release's whole `IfcShapeModel` family.
  - `Dim` follows each release's `IfcCurveDim` and IFC4X3's
    `IfcSegmentDim`: the IFC4X1-on and IFC4X3 curve families
    (`IfcOffsetCurveByDistances`, `IfcGradientCurve`, `IfcPolynomialCurve`,
    `IfcSpiral`, ...) and an `IfcCurveSegment`'s `ParentCurve` (slot 4)
    resolve, so the dimensional rules reach IFC4X3 alignment geometry, and
    `IfcGridAxis.WR1` derives `AxisCurve.Dim` the same way. In
    `IfcGetBasisSurface`, an IFC4X3 `IfcCurveSegment` contributes its
    `ParentCurve`'s surface, where slot 2 was read and an
    `IfcCompositeCurveOnSurface` of curve segments was reported as
    `SameSurface`.
  - A build that leaves the declared release's table out answers subtype
    tests from the compiled geometry chains and does not report a rule that
    demands a type for an entity those chains cannot classify.

## [0.14.0] - 2026-10-09

Semver: `IfcPolygonalBoundedHalfSpace` boundaries this crate refused as
`Unsupported` now lower, and a malformed composite or indexed boundary is
refused as `Degenerate` (an empty composite, a gap, an open curve, a point
off the plane) instead of `Unsupported`. No public item is added or
removed, and `PARTIAL` gains two rows; behaviour changes as it did for
#335, so the next release is a minor one (0.14.0).

### Added

- `IfcCompositeCurve` and `IfcIndexedPolyCurve` as
  `IfcPolygonalBoundedHalfSpace.PolygonalBoundary` (#393). `BoundaryType`
  admits `IfcCompositeCurve` in IFC2X3, IFC4 ADD2 TC1 and IFC4X3 ADD2 and
  `IfcIndexedPolyCurve` in IFC4X3 ADD2; an IFC4 file using the latter lowers
  the same way, the curve being defined identically there, and the rule is
  left to validation. Both are read by the profile boundary readers
  (#43's composite walk, #335's indexed reading) under a half-space role,
  so `SameSense`, nested composites, trimmed `IfcLine` segments, the
  collinear-arc fallback ("treated as a polyline segment") and the closure
  rules are shared, and the closed, straight-edged result becomes the one
  `Curve2::Polyline` with `closed` set that `SolidOperation::BoundedHalfSpace`
  takes. Composite joints are judged within the model's `Precision`
  ("the tolerance under which two given points are still assumed to be
  identical", `1.E-5` project units when none is declared), and a point
  keeps the `IfcPolyline` boundary's rule: 2D, or 3D with `z == 0` exactly,
  in `Position`'s XY plane (`BoundaryDim`). Refused, naming the entity:
  as `Degenerate`, a gap between segments, an open curve, an empty
  composite, a point off the plane and the indexed refusals of #335; as
  `Unsupported`, a circular arc (a trimmed `IfcCircle` or a non-collinear
  `IfcArcIndex`), because Axiolid's `BoundedHalfSpace` takes only a
  `Polyline2` (`axiolid-mesh-compile` 0.3.14 refuses every other `Curve2`,
  and `Curve2` has no composite variant) and an arc is never polygonised;
  an IFC4X3 `IfcCurveSegment` member; and other segment parents. The
  `IfcPolyline` boundary is unchanged. `PARTIAL` lists the family's
  admitted and refused boundary forms. The fixture
  `halfspace_boundaries_ifc4x3.ifc` clips a wall by each form beside its
  `IfcPolyline` twin; each lowers to the twin's polyline and compiles with
  `compile-reference-backend` to its volume.

## [0.13.0] - 2026-10-09

Semver: no public item is added, removed or changed in signature, and the
net geometry is the same up to floating-point rounding. But the neutral
graph `lower_product_net`/`lower_product_net_with` return is shaped
differently, and the module documentation described that shape: a caller
that matches `NetLowering::root` or a `Subtraction::result` as a `Boolean`,
or reads a `Subtraction::body` as the opening's placed solid, now finds an
`Instance` above it. Hosts the kernel refused also mesh now. That is an
observable behaviour change for code that inspects the graph, so the next
release is a minor one (0.13.0), not a patch.

### Changed

- Net lowering subtracts openings in the host's frame, not in world
  coordinates (#388). The host's Body is lowered without its world
  placement, each opening's Body by its placement relative to the host's,
  the `Difference` nodes run in that frame, and one `Instance` with the
  host's world transform (context `WorldCoordinateSystem` included, applied
  once) is put above `NetLowering::gross`, each `Subtraction::body` and each
  `Subtraction::result`; all of them stay in world coordinates. A host of
  several solids has each part cut in that frame. The relative placement is
  composed along the `IfcLocalPlacement` chain the opening and the host
  share, from their first common placement down, so a georeferenced site's
  translation and turn never enter it; a grid or linear placement may be
  that common placement or sit above it. When an opening shares no chain
  with its host (no `PlacementRelTo` path to a common placement, or a Body
  context whose frame differs from the host's), the net body is lowered in
  world coordinates exactly as before, rather than guessed.

### Fixed

- A wall with openings flush with both its faces, under a site placed at
  survey coordinates (600 000, 5 600 000) and turned to grid north, meshes
  net with `compile-reference-backend` (#388). In world coordinates the
  opening's faces were rounded off the wall's by up to an ulp of 5.6e6, and
  `axiolid-mesh-compile` 0.3.14 refused the result as touching itself along
  the opening's edges. The wall of `ifclite-geometry/issue_098_wall_W.ifc`,
  refused that way before, nets to 32.419 m³ now. Net volumes over the rest
  of the fixtures and the reference corpus are unchanged to 1e-8 m³.

## [0.12.0] - 2026-10-08

Semver: profile boundaries this crate refused as `Unsupported` now lower,
a malformed `IfcIndexedPolyCurve` boundary is refused as `Degenerate`
instead of `Unsupported`, and `profile_outline` refuses open indexed
curves it used to close. No public item is added or removed, but behaviour
changes, so the next release is a minor one (0.12.0).

### Added

- `IfcIndexedPolyCurve` as the `OuterCurve` and `InnerCurves` of
  `IfcArbitraryClosedProfileDef` and `IfcArbitraryProfileDefWithVoids`
  (#335), per IFC4 ADD2 TC1 and IFC4X3 ADD2: an `IfcLineIndex` becomes one
  straight `Line2` edge per consecutive index pair, an `IfcArcIndex` the
  exact `Circle2` through its three points with an angle domain, and a curve
  without `Segments` one straight edge per consecutive point pair. Nothing
  is chorded. The contour is the one the `IfcCompositeCurve` reader builds
  for the same outline (a trimmed `IfcCircle` for the arc), so both lower to
  the same `Profile`. An `IfcArcIndex` whose three distinct points are
  collinear lowers as the polyline start -> mid -> end (one edge when the
  middle point lies between the others, two when not), as the schema
  says: "the arc segment shall be treated as a polyline segment".
  Coincidence and collinearity are judged within the model's declared
  `Precision` (`1.E-5` project units when none is declared), "after taking
  the Precision factor into account". Refused as
  `GeometryError::Degenerate`, naming the curve: an open curve (closure by
  index with `Segments`, by first and last point coinciding within the
  `Precision` without), segments that break WHERE rule `Consecutive`, an arc
  with two coincident points, `SelfIntersect` TRUE, and a 3D point list.
  Other boundary curve families are still `Unsupported`.

### Changed

- `profile_outline` refuses an open `IfcIndexedPolyCurve` boundary as
  `GeometryError::Degenerate`, as the profile lowering does (#335). It
  closed one implicitly before, reporting a ring whose closing edge the file
  never authored. Closure is the schema's: by index with `Segments`, by the
  first and last point coinciding within the model's `Precision` without.
  `IfcPolyline` rings are unchanged.

### Fixed

- `IfcCurveBoundedPlane` boundaries given as an `IfcCompositeCurve` of
  polyline segments compile with `compile-reference-backend` (#336). They
  lower unchanged, as a `CurveRelation::Composite` whose segments keep
  their `SameSense`; `axiolid-mesh-compile` 0.3.12 refused that relation
  ("is not a curve node"), and 0.3.13, the floor this crate already
  requires, resolves `Composite` and `Trimmed` boundaries (axiolid/kernel#255).
  No merge into one polyline is done here, so the segment structure stays
  in the neutral graph. A test now pins it: three segments with one
  reversed around a two-segment hole, and one reversed segment wrapping a
  clockwise ring, compile to their exact areas.

## [0.11.0] - 2026-10-08

Placement resolution covers every `IfcObjectPlacement` kind: linear
placements compose with their alignment's frame (#357), grid placements
resolve (#362), and any placement may be relative to a linear or grid one
(#363). Error variants are added and resolved positions change for
off-identity alignments and grid-placed products, so the next release is a
minor one (0.11.0).

### Added

- `IfcGridPlacement` resolution (#362), in `product_world_transform`,
  `products_world_transforms`, `PlacementResolver::world_transform` and
  product lowering. Per IFC4.3 ADD2: the location is the intersection of
  the two axes' offset curves, `OffsetDistances[1..2]` to the LEFT of each
  axis ("anti-clockwise rotation through 90 degrees from the tangent"),
  reverted by `IfcGridAxis.SameSense`, and `OffsetDistances[3]` along the
  grid's Z; the x-axis is the first axis's tangent, an `IfcDirection`'s x
  and y ratios, or the direction towards a second
  `IfcVirtualGridIntersection`; z is the grid's Z; all of it in the frame of
  the `IfcGrid` that lists the axes, whose `ObjectPlacement` composes above.
  Straight axes (`IfcLine`, a collinear `IfcPolyline`, an `IfcTrimmedCurve`
  on a line, an `IfcOffsetCurve2D` of one) intersect in closed form. Curved
  axes (`IfcCircle`, `IfcEllipse`, a bent `IfcPolyline`, an
  `IfcTrimmedCurve` on a conic with parameter trims, an `IfcOffsetCurve2D`
  of one) intersect through the caller's `CurveEvaluator`
  (`LoweringSession::with_curve_evaluator`,
  `product_world_transform_with_evaluator`, feature `compile`); without
  one they are refused as `Unsupported` naming the axis curve. Both the
  IFC2X3/IFC4 layout of `IfcGridPlacement` and IFC4X3's, which prepends the
  inherited `PlacementRelTo`, are read; `GridPlacement::placement_rel_to`
  is new.
- `GeometryError::GridAxesParallel { intersection, axes }`,
  `GridAxesDoNotIntersect { intersection, axes, detail }` (curved axes that
  miss within their extent or meet more than once),
  `GridAxisWithoutGrid { axis }`, `GridAxesInDifferentGrids { intersection,
  grids }`, and `PlacementRelToConflict { placement, stated, implied }`: a
  stated `PlacementRelTo` that resolves to a different frame than the
  alignment's or grid's `ObjectPlacement` IFC4.3 says it references. A grid
  without `ObjectPlacement` is `MissingAttribute`.

### Changed

- An `IfcLinearPlacement` is placed in the frame its basis curve is stated
  in (#357), through both the cached `CartesianPosition` and the derived
  path: its `PlacementRelTo` when stated, otherwise the `ObjectPlacement` of
  the `IfcAlignment` (or other product) whose representation carries the
  basis curve. IFC4.3 ADD2 concept Product Linear Placement: "each product
  placement that uses Product Linear Placement references the
  IfcObjectPlacement of the IfcLinearPositioningElement through
  IfcLinearPlacement.PlacementRelTo"; `IfcObjectPlacement.PlacementRelTo`:
  "If it is omitted, then in the case of linear placement it is
  established by the origin of horizontal alignment of the referenced
  IfcAlignment Axis". `CartesianPosition`, a fallback for
  `RelativePlacement`, is read relative to the same frame. Products on an
  alignment placed off identity used to be placed as if the alignment sat
  at the origin, and `CachedPositionPolicy::Verify` refused their correct
  caches; both are fixed. `CachedPlacementMismatch` reports world
  positions. The context's `WorldCoordinateSystem` is still applied once,
  by the representation frame. `derive_linear_placement_transform` returns
  the frame in the basis curve's coordinates, as before; its documentation
  now says so.
- `PlacementResolver::world_transform` walks `PlacementRelTo` through
  local, grid and linear placements alike (#363), caching each and keeping
  the cycle and depth refusals across mixed chains. A local placement
  relative to a linear placement used to be `WrongEntityType`, and one
  relative to a grid placement `Unsupported`. An entity that is no
  `IfcObjectPlacement` reports `expected: "IfcObjectPlacement"`.

### Added (#328, geometry in the bindings)

- `compile::Tolerance` and `compile::TriMesh` re-export the tolerance every
  compile entry point takes and the mesh it returns, so a caller that
  names no `axiolid-*` crate (the `openbim-ifc` facade's `mesh` feature)
  can call them. Behind `compile`, as before.

Semver: additive; it ships with the minor release above (0.11.0), which
the `openbim-ifc` facade's `mesh` feature needs.

## [0.10.0] - 2026-10-04

Product lowering gains two opt-ins: Reference View openings taken as applied
(#351) and an injected curve evaluator for linear placements (#353), which
also checks a cached position (#354). The derived linear-placement frame now
follows IFC4.3 (#355), which changes `derive_placement_transform`'s output.
API is added and behaviour changes, so the next release is a minor one
(0.10.0).

### Added

- Net lowering of hosts whose IFC4 Reference View openings carry no Body
  (#351). `lower::lower_product_net_with(session, product, NetOptions)`,
  with `NetOptions::default().with_reference_only_openings(
  ReferenceOnlyOpenings::TakeAsApplied)`, lists an `IfcOpeningElement` whose
  representations are all `Reference` in the new
  `NetLowering::taken_as_applied` (`TakenAsApplied { opening, relation,
  reason: AppliedReason::ReferenceRepresentationOnly }`) and subtracts
  nothing for it; a host whose openings are all taken as applied lowers to
  its gross Body. IFC4 ADD2 TC1 `IfcOpeningElement`: a `'Reference'`
  representation "is not subtracted, it is provided in addition to the hole
  in the Body shape representation of the voided element", and its
  Reference View concept says it "shall not be used to subtract the
  opening". `lower_product_net` is unchanged and still refuses such a host
  (ADR 0014, amended). An opening with no representation, one with another
  representation beside `Reference`, a `Reference`-only `IfcVoidingFeature`
  and an opening whose Body does not lower are refused whatever the option.
- Evaluator-taking product lowering (#353), feature `compile`:
  `LoweringSession::with_curve_evaluator(&dyn CurveEvaluator)` places an
  `IfcLinearPlacement` without a cached `CartesianPosition` by deriving it
  from its `RelativePlacement` through the caller's evaluator, for every
  entry point that takes the session (`lower_product_items`,
  `lower_product_representation`, `lower_product_net` and its openings).
  The same for callers without a session:
  `product_world_transform_with_evaluator`,
  `product_representation_frame_with_evaluator`, and
  `constraint::placement::derive::derive_linear_placement_transform` for one
  placement. An `IfcParameterValue` on an alignment centreline is refused
  by name (#347). Without an evaluator nothing changes: the cache is read,
  and a placement without one is refused.
- Cached-position check (#354), feature `compile`: with an evaluator, a
  cached `CartesianPosition` is compared with the derived location under
  `CachedPositionPolicy::Verify` (the default). Farther apart than the
  model's tolerance is the new `GeometryError::CachedPlacementMismatch {
  placement, cached, derived, distance, tolerance }`; within it, the
  derived frame is used, since IFC4.3 makes the cache "an optional
  fallback" for the linear expression. Only the location is compared.
  `CachedPositionPolicy::Trust` (`LoweringSession::
  with_cached_position_policy`) uses a cache as it is and derives only
  uncached placements, for a model whose expressions this bridge cannot
  derive. The tolerance, `derive::cached_position_tolerance`, is the
  coarsest `Precision` of the model's 3D contexts, which IFC defines as
  "the tolerance under which two given points are still assumed to be
  identical", in the project length unit and converted to metres; IFC's
  default of 1e-5 project units when none is declared; floored at
  floating-point rounding (1e-9 relative, as the alignment seams).
- `LoweringSession::derives_linear_placements`.

### Changed

- `derive_placement_transform` builds the IFC4.3 frame from the evaluator's
  point and tangent (#355): axes `(tangent, left, up)`, where `left` is the
  horizontal `Z x tangent` and `up` is perpendicular to the tangent in its
  vertical plane. A positive `OffsetLateral` now moves LEFT, as
  `IfcPointByDistanceExpression` states; it moved right, and the product's
  local Z lay along the lateral. `derive_linear_placement_transform` also
  composes an explicit `IfcAxis2PlacementLinear.Axis`/`RefDirection` in that
  frame (8.9.3.4), which was ignored.
- `derive_placement_transform` refuses a distance when the evaluator's
  `distance_convention` for the basis curve is not the one IFC states:
  plan distance on an `IfcGradientCurve` or `IfcAlignment`, arc length on a
  polyline. The reference evaluator agrees on all of them.
- The refusal of an uncached `IfcLinearPlacement` names the evaluator-taking
  entry points.

## [0.9.0] - 2026-10-04

Input this crate refused now lowers exactly onto the station relations of
axiolid-model 0.3.5 (ADR 0082), and a parameter along an alignment that a
newer evaluator answered is refused. Behaviour changes and API is added, so
the next release is a minor one (0.9.0).

### Added

- IFC4X3 stations (#307), stored exactly, never evaluated:
  - `IfcPointByDistanceExpression` lowers to `GeometryNode::CurveStation`.
  - `IfcAxis2PlacementLinear` lowers to `GeometryNode::OrientedCurveStation`.
    `Axis` and `RefDirection` pass through as components in the curve's
    (tangent, left, up) frame, which is how IFC4.3 ADD2 8.9.3.4 reads them.
  - `IfcOffsetCurveByDistances` lowers to `CurveRelation::OffsetByStations`.
    Offsets that stop short of the ends continue unchanged to them, as
    8.9.3.42.3 states, through added stations when the basis states its
    length.
  - `IfcSectionedSolidHorizontal` lowers to
    `SolidOperation::SectionsAtStations`, and `IfcSectionedSurface` to
    `SurfaceRelation::OpenSectionsAtStations`, with the
    `IfcOpenCrossProfileDef.Tags` as section tags (matched as sets, an
    open section forwards or reversed).
  - New entry points: `lower::lower_point_by_distance_node` and
    `lower::lower_axis2_placement_linear_node` (module `lower::station`).
  - The five families move from `dispatch::PLANNED` to `IMPLEMENTED`, with
    `PARTIAL` rows for their refusals. `BodyKind::classify` gives
    `IfcSectionedSolidHorizontal` `SectionedSpine` and the two station items
    `Point`.
- The conventions match IFC4.3 ADD2 exactly, so nothing is converted:
  - `DistanceAlong` is plan distance on an `IfcGradientCurve` (its
    parameter is the BaseCurve's, 8.9.3.34.1) and arc length elsewhere.
  - `OffsetLateral` is positive to the left.
  - `OffsetVertical` is perpendicular to the tangent in its vertical plane
    (`StationFrame::Section`).
  - `OffsetLongitudinal` runs along the tangent.
- Section profile axes follow the documented upstream fix, not the printed
  ADD2 sentences: profile Y = `Axis`, normal = `RefDirection`, profile
  X = `Axis x RefDirection` (the left lateral by default). The sources are
  buildingSMART/IFC4.x-IF#147, IFC4.x-development#1010 and #1151, PRs
  #1162 and #1163, and `IfcOpenCrossProfileDef` 8.15.3.15.1. 8.8.3.35.1
  says profile X is `RefDirection`, and 8.8.3.37.1 says X is
  `Directrix x Axis`, to the right. #344 tracks the upstream change.

### Refused by name

- `DistanceAlong` as `IfcParameterValue`, and a distance before the start
  or beyond a stated length.
- A station on a tangent discontinuity of its basis, and a run of
  sections or offsets across one. IFC lets the previous segment's tangent
  govern (8.9.3.48.3) and mitres sections (8.8.3.35.1). The neutral
  evaluators read the next segment's tangent and do not mitre. Seams are
  read from stored data: polyline corners, and elevation-law breaks where
  the grade jumps. A basis whose seams cannot be located is refused: a
  curve relation, or a B-spline with a knot of multiplicity at least its
  degree.
- A frame that scales, mirrors or tilts the vertical. WR2 (parallel
  axes). 2D axis directions.
- On an offset curve: offsets short of an unbounded or unstated end, a
  non-zero `OffsetLongitudinal`, and a member on another basis.
- On the sections: the WHERE rules `NoLongitudinalOffsets`, `NoOffsets`,
  `CorrespondingSectionPositions` and `SectionsSameType`, positions off
  the directrix or out of order, mixed tagging, and branching breaklines.

### Fixed

- `constraint::placement::derive::derive_placement_transform` refuses an
  `IfcParameterValue` `DistanceAlong` on an `IfcGradientCurve` or
  `IfcAlignment` basis curve by name (`GeometryError::Unsupported` on the
  basis curve) and no longer hands it to the injected `CurveEvaluator`
  (#347). IFC4.3 ADD2 does not define that parameter: a gradient curve takes
  its `BaseCurve`'s (8.9.3.34.1), a composite accumulates the parametric
  ranges of its parent curves (8.9.3.20.1), which are angles for a circle
  (8.9.3.18.1) and `u = s / (A sqrt(pi))` for a clothoid (8.9.3.19.1), and
  `IfcCurveSegment` says no parametric space is yet defined for its parent
  curves (8.9.3.28.1). The station lowering above refuses it the same way.
  A parameter on an `IfcPolyline` or line-only `IfcIndexedPolyCurve`, where
  IFC counts one per segment (8.9.3.51), is unchanged.
- Released 0.3.0 through 0.8.1 passed that parameter through as the
  evaluator's native parameter. With `axiolid-evaluate` up to 0.3.5 the
  reference evaluator refused it, so nothing was placed. With
  `axiolid-evaluate` 0.3.6 (released 2026-10-04), which any of those
  releases resolves to, it reads the parameter as plan distance (axiolid
  ADR 0082) and answers: a product placed by an `IfcParameterValue` along an
  alignment lands at that plan distance, a position IFC does not give it.
  Another injected evaluator received the same undefined value. Upgrade to
  get the refusal.

### Changed

- `Cargo.lock` takes `axiolid-construct` 0.3.15, `axiolid-mesh-compile`
  0.3.14 and, dev-only, `axiolid-evaluate` 0.3.6 and `axiolid-reference`
  0.3.7. The workspace minimums below stay: nothing here relies on the new
  versions.
- The workspace requires `axiolid-model` 0.3.5. With
  `compile-reference-backend` it requires `axiolid-mesh-compile` 0.3.13 and
  `axiolid-construct` 0.3.14, which resolve and mesh the station relations.
  Dev-only: `axiolid-evaluate` 0.3.5 and `axiolid-reference` 0.3.6.
- An `IfcCurveSegment` placed by an `IfcAxis2PlacementLinear` is still
  refused. Its station lowers now, but no neutral relation places a curve
  in a station's frame (#311). `IFCSEGMENTEDREFERENCECURVE` stays in
  `PLANNED` (#311).

## [0.8.1] - 2026-10-04

No public API changes. The reference backend's floor rises and input it
refused now compiles, as with earlier kernel floors (0.3.1): the next release
is a patch (0.8.1).

### Changed

- `compile-reference-backend` requires `axiolid-mesh-compile` 0.3.12 and
  `axiolid-construct` 0.3.13 (axiolid/kernel#245, #315). An
  `IfcCurveSegment` over a 2D `IfcPolynomialCurve` (the `CUBIC` transition,
  #90) lowers to a Bezier trimmed at `TrimSelector::ArcLength`. 0.3.4 read
  parameter selectors only and refused that trim by name; 0.3.12 resolves it
  by quadrature, so the segment compiles. A swept disk whose polyline
  directrix turns a sharp corner without a fillet radius compiles with an
  exact half-angle mitre, so its volume is the section area times the
  centreline length. 0.3.9 to 0.3.11 refused that corner, so a downstream
  build that resolved them fresh refused such pipes. The kernel still
  refuses three cases by name (axiolid/kernel#248): a sharp corner beside an
  arc, a closed polyline, and `FilletRadius` equal to `Radius`.

## [0.8.0] - 2026-10-03

Input this crate refused now lowers exactly, onto the Axiolid relations
of axiolid-curve 0.3.3 and axiolid-model 0.3.4: behaviour changes, so the
next release is a minor one (0.8.0).

### Changed

- An `IfcCurveSegment` over a 2D `IfcPolynomialCurve` (the `CUBIC`
  transition) lowers to its exact Bezier, placed rigidly and trimmed at
  `TrimSelector::ArcLength(SegmentLength)`, instead of refusing (#90). A
  non-zero `SegmentStart`, a backwards walk, a 3D polynomial, and one with
  no degree-one coordinate to bound the trim stay refused by name.
- An `IfcGradientCurve` whose base curve holds such a segment has a
  `Curve2::Chain` plan: the polynomial is a parametric piece read by arc
  length (#90).
- A vertical `IfcCircle` segment in an `IfcGradientCurve` lowers to
  `ElevationLaw::CircularArc`, and a vertical `IfcSpiral` (the
  `IfcClothoid`) to `ElevationLaw::Intrinsic`, its extent read from the
  next segment's start (#258).
- `dispatch::PLANNED` keeps `IFCPOLYNOMIALCURVE` (unbounded on its own; it
  lowers as an `IfcCurveSegment` parent) and `IFCSEGMENTEDREFERENCECURVE`,
  now citing #311: the roll law exists, but the geometric form states cant
  through stations (#307) and parent curves with no normative mapping. An
  `IfcAxis2PlacementLinear` placement now cites #307.

## [0.7.0] - 2026-10-03

`select::subtype` carries IFC4X3 ADD2 supertype chains (#293). Answers
change for IFC4X3-only entity names, and a build without default features
links fewer schema tables (#306), so the next release is a minor one.

### Changed (breaking)

- Release features `ifc2x3`, `ifc4`, `ifc4x1`, `ifc4x2` and `ifc4x3`, all
  default, forward to `ifc-schema`, which this crate now depends on without
  its default features (#306). Each links one release's table. The default
  build links every release, as before, but a build without default
  features (the kernel-free column, `openbim-ifc`'s `geometry-select`) now
  links only the releases it names; release-bound authoring refuses the
  others with `GeometryError::AuthoringSchemaUnbound`. Representation
  selection reads no table and is unaffected. `lowering` still links
  every release through `ifc-alignment`.

### Added

- `select::is_a_in`, `supertypes_of_in` and `known_entities_in` answer for
  one named release, and `VERIFIED_SCHEMA_VERSIONS` lists the releases they
  answer verbatim: IFC4 ADD2 TC1 and IFC4X3 ADD2.
  `tables_are_verified_for(Ifc4x3)` is now `true`.
- The IFC4X3 rows are a delta over the IFC4 tables: the 19 concrete
  geometry, profile and placement entities IFC4X3 adds, the abstract
  supertypes they introduce (`IfcSpiral`, `IfcSegment`, `IfcOffsetCurve`,
  `IfcDirectrixCurveSweptAreaSolid`, `IfcSectionedSolid`), and the IFC4X3
  chains of the four IFC4 entities IFC4X3 re-parents.
  `tests/schema_coverage.rs` compares every chain, per release, with
  `IFC4.exp` and `IFC4X3_ADD2.exp`.

### Changed

- The release-neutral `is_a` and `supertypes_of` now resolve IFC4X3-only
  entities with their IFC4X3 chain: `is_a("IFCCLOTHOID", "IFCCURVE")`,
  `is_a("IFCGRADIENTCURVE", "IFCCOMPOSITECURVE")`,
  `is_a("IFCTRIANGULATEDIRREGULARNETWORK", "IFCTESSELLATEDFACESET")` and
  `is_a("IFCOPENCROSSPROFILEDEF", "IFCPROFILEDEF")` were `false` and are
  `true`. Every answer for an entity IFC4 declares is unchanged, including
  the four IFC4X3 re-parents (`IfcOffsetCurve2D`, `IfcOffsetCurve3D`,
  `IfcFixedReferenceSweptAreaSolid`, `IfcSurfaceCurveSweptAreaSolid`), which
  keep their IFC4 chain unless a caller asks `is_a_in(Ifc4x3, ..)`.
- Visible effects: the select resolvers (`GeometricSetSelect`,
  `BooleanOperand`, `PointOrVertexPoint`, ...) accept IFC4X3-only members
  instead of reporting a wrong type. `BodyKind::classify` returns `Curve`
  for the six `IfcSpiral` subtypes, `IfcPolynomialCurve`,
  `IfcOffsetCurveByDistances` and `IfcSegmentedReferenceCurve`, and
  `Surface` for `IfcSectionedSurface`, so body description lists those
  items instead of refusing them. Lowering is unchanged: those families
  still lower, or refuse with the `dispatch::PLANNED` reason, as before.
  Where-rules that test inheritance (dimensionality, swept-area rules) now
  also apply to the IFC4X3 subtypes the schema makes them inherit.

## [0.6.1] - 2026-10-03

IFC4X3 ADD2 geometry families (#243). Every concrete IFC4X3 representation
item and profile is now lowered or refused with a named reason.

### Added

- IFC4X3 alignment curves lower exactly. `IfcCurveSegment` lowers
  for `IfcLine`, `IfcCircle`, 2D `IfcPolyline` and all six `IfcSpiral`
  parents, placed by its `Placement` and cut by its `IfcLengthMeasure`
  arc lengths: a polyline, an angle-trimmed circle, or a planar
  `Curve3::Intrinsic` carrying the spiral's curvature law rebased to the
  segment in closed form. A negative `SegmentLength` walks the parent
  backwards; the zero-length closing segment is its placement, and a
  composite drops it. `IfcCompositeCurve` now accepts `IfcCurveSegment`
  members.
- The spiral laws read the IFC4.3 terms: `sign(A_n) s^n / |A_n|^(n+1)` for
  the clothoid and the second-, third- and seventh-order polynomial
  spirals; `1/A_0 + cos(pi s/L)/A_1` (cosine) and
  `1/A_0 + sign(A_1) s/A_1^2 + sin(2 pi s/L)/A_2` (sine), with `L` the
  using segment's length. A present zero term is refused, not read as
  absent.
- `IfcGradientCurve` lowers to `Curve3::Elevated`: its `BaseCurve` as one
  intrinsic plan with a piecewise curvature law, its vertical segments
  (`IfcLine` grades, degree-2 `IfcPolynomialCurve` parabolas) as a
  piecewise elevation law. Seams are checked in closed form: headings
  everywhere, positions after a line or arc, parabola arc lengths.
- IFC4X3 `IfcOpenCrossProfileDef` lowers exactly through
  `lower_open_profile_node` to an open polyline whose vertices are the
  closed-form sums of its widths and slopes (horizontal or along-slope
  widths, `OffsetPoint` honoured, slopes measured from +X towards +Y as the
  IFC4.3 figure states). `describe_profile` reads it as the new
  `ProfileParameters::OpenCross`, in metres and radians, refusing a
  `Widths`/`Slopes` or `Tags` count mismatch, a negative width and a vertical
  slope with horizontal widths. It bounds no area, so the area-profile path
  refuses it, as for `IfcArbitraryOpenProfileDef`. `section_slot::OC_*` are
  its slot constants.
- IFC4X3 `IfcDirectrixDerivedReferenceSweptAreaSolid` lowers to the same
  exact `FixedReferenceSweep` as its supertype when the directrix defines only
  a tangent, which IFC4.3 says is the identical behaviour. A directrix that
  defines a tangent plane (built from `IfcCurveSegment`s, such as
  `IfcGradientCurve` and `IfcSegmentedReferenceCurve`, or lying on a surface)
  is refused as `Unsupported` naming the missing neutral primitive, the
  public `lower::swept::DIRECTRIX_DERIVED_TANGENT_PLANE`.
- `IfcTriangulatedIrregularNetwork` (IFC4X3) lowers as a triangle mesh when
  every `Flags` value is a breakline code (0 to 7). A void or hole triangle
  (-2, -1) or an undocumented code is a typed `Unsupported` refusal; a
  `Flags` list whose length differs from the triangle count is `Degenerate`.
  `solid::tessellated::TriangulatedIrregularNetwork` is the borrowed view.
  `BodyKind::classify` and `SolidKind::classify` report it as tessellated.
- `lower::dispatch::SPECIALISATIONS` (and `Specialisation`): subtypes routed
  to their supertype's lowering, each naming the attributes it adds. A test
  checks every row against the IFC4X3 schema.
- `tests/schema_coverage.rs` also enumerates IFC4X3 ADD2. Its inventory is
  checked against the bundled table and `IFC4X3_ADD2.exp`. Every IFC4X3
  representation item must be in `IMPLEMENTED`, in `PLANNED`, or have a
  nested disposition. Every IFC4X3 profile must be read or in `UNLOWERED`.
- Fixtures `synthetic_ifc4x3_alignment_curves.ifc` and
  `synthetic_ifc4x3_geometry_families.ifc`, with their generators
  `tools/gen_ifc4x3_curve_fixtures.py` and
  `tools/gen_ifc4x3_geometry_fixtures.py`.

### Changed

- `lower::dispatch::PLANNED` lists the IFC4X3 representation items that are
  not lowered, each with its runtime refusal text, instead of the generic
  "representation item family is not lowered yet":
  - a spiral or `IfcPolynomialCurve` on its own (unbounded; each lowers as
    the `ParentCurve` of an `IfcCurveSegment`);
  - `IfcSegmentedReferenceCurve` (cant has no neutral roll law, #93);
  - `IfcSectionedSolidHorizontal` and `IfcSectionedSurface` (no neutral
    sectioned sweep over curve-measure stations, nor a sectioned-surface
    relation);
  - `IfcOffsetCurveByDistances`, `IfcPointByDistanceExpression` and
    `IfcAxis2PlacementLinear` (no neutral distance-along-curve relation).
- Further typed refusals, recorded in `dispatch::PARTIAL`: an
  `IfcPolynomialCurve` parent trimmed by arc length and a parabola without a
  stated end (#90); `IfcAxis2PlacementLinear` segment placements (#93);
  vertical arcs and clothoids (#258); `IfcParameterValue` measures; plan
  kinks, gaps and profiles that do not span the plan.

### Fixed

- An `IfcCurveSegment` reached through a path that cannot lower it (a sweep
  `StartParam`/`EndParam` range, a p-curve, a profile boundary) is refused
  by name. Before, the `IfcCompositeCurveSegment` slots were read, so
  `SegmentLength` was taken for `ParentCurve` and the result was a
  misleading wrong-value-kind error. `curve::composite::CURVE_SEGMENT_UNREAD`
  is the view's backstop refusal.
- `constraint::placement::derive_placement_transform` resolves an
  `IfcLinearPlacement` whose basis curve is an `IfcGradientCurve` through
  the gradient-curve lowering. Before, it passed the curve to
  `ifc_alignment::gradient_curve3`, which expects an `IfcAlignment`, so the
  path always refused.
- The `dispatch::PLANNED` documentation claimed every recognized
  representation item is lowered. It now lists the IFC4X3 families that are
  not.

## [0.6.0] - 2026-10-02

### Changed (breaking)

- Requires `ifc-alignment` 0.5. `derive_placement_transform` takes an
  `ifc_alignment::PointByDistance`, so the alignment version is part of
  this crate's public API; mixing it with `ifc-alignment` 0.4 types no
  longer compiles. No behaviour changes in this crate.

## [0.5.0] - 2026-09-29

### Changed

- `MaterialProfileSetUsageGeometry::new` accepts
  `IfcMaterialProfileSetUsageTapering`, the schema subtype of
  `IfcMaterialProfileSetUsage`, whose inherited slots it reads unchanged;
  the new `MaterialProfileSetUsageGeometry::tapering()` returns its
  `MaterialProfileSetUsageTaperingGeometry` (end profile set and end
  cardinal point), or `None` for a plain usage (#136).

### Changed (breaking)

- `ViolationKind`, `Support` and `FunctionStatus` are `#[non_exhaustive]`: a
  match needs a wildcard arm.
- `RuleViolation`, `LoweredGeometry` and `MappedInstance` are
  `#[non_exhaustive]`; they can no longer be built with a struct literal
  outside the crate.
- `authoring::surface_curve_swept_area_solid`,
  `authoring::fixed_reference_swept_area_solid` and
  `authoring::swept_disk_solid` are removed (#210). Without the model they
  could not write every release correctly: the two directrix sweeps wrote
  the IFC4X3 `IFCPARAMETERVALUE(..)` trim into IFC4 files, and the swept
  disk wrote `$` for the trim IFC2X3 requires. Use
  `surface_curve_swept_area_solid_in`, `fixed_reference_swept_area_solid_in`
  and the new `swept_disk_solid_in`, which take `&Model` after the
  transaction and otherwise the same arguments.
- `SurfaceCurveSweptAreaSolid::start_param`/`end_param` and
  `FixedReferenceSweptAreaSolid::start_param`/`end_param` return
  `GeometryResult<Option<TrimMeasure>>` instead of `Option<f64>` (#210). In
  IFC4X3 the trim is an `IfcCurveMeasureSelect`, and an
  `IFCLENGTHMEASURE(..)` trim is a distance along the directrix, not a curve
  parameter; the reader now says which (`TrimMeasure::Parameter` or
  `TrimMeasure::Length`) instead of returning both as a parameter. A bare
  number is a parameter, as IFC2X3 and IFC4 declare. A typed value that is
  not one of the SELECT's members is refused with `WrongValueKind` instead
  of being unwrapped.
- Lowering an `IfcSurfaceCurveSweptAreaSolid` or
  `IfcFixedReferenceSweptAreaSolid` whose trim is an `IfcLengthMeasure`
  fails with `Unsupported` (#210). It used to pass the length on as a curve
  parameter, which on a conic directrix reads metres as radians; converting
  a length into the directrix's parameter needs arc-length evaluation,
  which lowering does not do.

### Added

- `authoring::swept_disk_solid_in` (#210): an `IfcSweptDiskSolid` in the
  model's declared release. The trim is written bare in every release, as
  before; in IFC2X3, which declares `StartParam` and `EndParam` required,
  an unset one is refused with `InvalidAuthoredValue` and nothing is
  staged.
- `solid::swept::TrimMeasure` (re-exported from `solid`), the kind and
  value of a directrix sweep's trim, and `TrimMeasure::parameter`.

### Fixed

- `authoring::curve_segment` writes a `CurveMeasure::Length` as
  `IFCLENGTHMEASURE(..)` (#210). It wrote `IFCNONNEGATIVELENGTHMEASURE(..)`,
  which is not a member of IFC4X3 `IfcCurveMeasureSelect =
  SELECT (IfcLengthMeasure, IfcParameterValue)` and is an `ifc-validate`
  error. Because `IfcLengthMeasure` is signed and neither slot is bounded,
  a negative length is now written as given instead of refused.

### Changed

- Depends on `ifc-schema` with its default features named explicitly
  (every bundled release), now that the workspace dependency turns them
  off for the facade's per-release features (#112).
- A model whose header declares `IFC4X1` or `IFC4X2` is refused with the
  existing unsupported-schema error. `ifc-schema` now bundles both
  releases, but no layout here is verified against them, so they are
  never read as IFC4 or IFC4X3.

## [0.4.4] - 2026-09-28

### Added

- `authoring::surface_curve_swept_area_solid_in` and
  `authoring::fixed_reference_swept_area_solid_in` (#200). They take the
  model and write `StartParam`/`EndParam` in the form its declared release
  requires: bare in IFC2X3 and IFC4, where the slot is `IfcParameterValue`,
  and `IFCPARAMETERVALUE(..)` in IFC4X3, where it is the SELECT
  `IfcCurveMeasureSelect`. The release binds from `FILE_SCHEMA` as the
  other authoring crates bind it (none binds IFC4). An attribute the release
  requires left unset (IFC2X3 `Position`, `StartParam`, `EndParam`) is
  refused with `InvalidAuthoredValue`.
- `GeometryError::AuthoringSchemaUnbound` (an unknown or ambiguous
  `FILE_SCHEMA`) and `GeometryError::AuthoringEntityNotInSchema` (the
  release does not declare the entity, such as the fixed-reference sweep in
  IFC2X3), for those writers. `GeometryError` is `#[non_exhaustive]`, so
  this is not breaking.
- `authoring::grid_with_owner_history` (#202): an `IfcGrid` in the model's
  declared release, with a caller-supplied `IfcOwnerHistory`, which IFC2X3
  requires on every `IfcRoot`. It binds the release as the `_in` writers
  above do, and lays the record out by attribute name from its table, so
  an IFC2X3 grid has its 10 attributes, not IFC4's 11. A `predefined_type`
  in IFC2X3, which declares none, or outside the release's
  `IfcGridTypeEnum`, is refused with `InvalidAuthoredValue`. The owner
  history must be in the model or staged on the transaction and be an
  `IfcOwnerHistory` (`InvalidAuthoredValue` on `OwnerHistory` otherwise);
  none is ever invented. IFC4 and IFC4X3 records are `grid`'s with the
  owner history in its optional slot. No error variant is added.

### Fixed

- `IfcParameterValue` slots are written bare (#200):
  `rectangular_trimmed_surface` (`U1`, `V1`, `U2`, `V2`), `point_on_curve`,
  `point_on_surface`, `reparametrised_composite_curve_segment`
  (`ParamLength`), and the `StartParam`/`EndParam` of `swept_disk_solid` and
  `swept_disk_solid_polygonal`. Each is declared with the defined type
  `IfcParameterValue`, not a SELECT, in every release that declares it, and
  ISO 10303-21 writes a typed parameter only for a SELECT. The readers
  accept both forms, as before.

### Changed

- `surface_curve_swept_area_solid` and `fixed_reference_swept_area_solid`
  still write `IFCPARAMETERVALUE(..)`, which is correct in IFC4X3 only. They
  cannot see the release; their docs now say so and point IFC4 (and IFC2X3)
  callers to the `_in` writers.
- `authoring::grid` is unchanged and documents its limitation (#202): it
  takes no model, so it writes the IFC4 layout (11 attributes,
  `OwnerHistory` `$`), which is never valid IFC2X3. It moved from
  `authoring/transform.rs` to `authoring/grid.rs`; the public path is the
  same.

## [0.4.3] - 2026-09-28

### Added

- `BodyItem::item_world`: the frame each described item is placed in, the
  context and product placement composed with every `MappingTarget o
  MappingOrigin` it was reached through (#185). It is reported for every item
  kind, including mapped B-reps and tessellations, and is not required to be
  rigid, so a mapping that mirrors or scales shows. `BodyItem::is_mirrored()`
  answers whether that frame reverses handedness, and
  `Transform::determinant()` gives its signed volume scale. Additive:
  `BodyItem` is `#[non_exhaustive]`.

## [0.4.2] - 2026-09-27

### Changed

- `data/ifc4-where-rules.tsv` lists each geometry WHERE rule by entity,
  label and support state only. Its `expression` column held the rule bodies
  verbatim, which are CC BY-ND schema text and are no longer shipped.
  `data/NOTICE.md` covers all five data files and no longer claims the
  directory holds no rule bodies while it did.

## [0.4.1] - 2026-09-27

### Fixed

- An `IfcCurveBoundedPlane` lowered under a non-identity frame no longer
  moves its boundaries twice (#163). `OuterBoundary` and `InnerBoundaries`
  are in the basis plane's parameter space, which is also how the neutral
  curve-bounded relation reads them, but the frame was applied to them as
  well as to the plane: they landed elsewhere or were refused as off the
  plane. Only the basis plane takes the frame now, so a framed plane meshes
  to the identity mesh moved by the frame. Connection surfaces lowered under
  a space's frame are the case this breaks.

### Added

- `product_representation_frame(model, units, product, purpose)`: the frame
  a product's representation of that purpose is placed in, the context's
  `WorldCoordinateSystem` composed above the placement chain (#164). Lowering
  and `body_description` now take their frame from it, so geometry lowered
  outside the body, such as a space boundary's connection surface in the
  relating space's coordinates, is placed exactly as the body is.
  `Ok(None)` when the product has no such representation; kernel-free.
- `profile_outline(model, units, profile)` and `ProfileOutline` (#166): an
  `IfcArbitraryClosedProfileDef`'s or `IfcArbitraryProfileDefWithVoids`'s
  boundaries as rings of vertices in metres, in profile coordinates, for
  `IfcPolyline` and line-only `IfcIndexedPolyCurve` boundaries. Each ring
  is in authored order without its closing vertex, as profile lowering
  reads it. An `IfcArcIndex` segment or any other curve family is
  `Unsupported` naming the curve, never chorded; a 3D point, fewer than
  three distinct vertices and non-consecutive segments are `Degenerate`.
  Kernel-free.

### Changed

- `lower_product_representation` selects the representation before it
  resolves the placement, as `body_description` already did: a product with
  no representation of the purpose is `Ok(None)` even when its placement is
  broken, where it used to be the placement error.

## [0.4.0] - 2026-09-27

### Fixed

- A face surface whose `FaceSurface` is dangling now reports the face as the
  referrer, and one naming a non-surface is `WrongEntityType` naming the
  target (#155). The first reported the missing id against itself; the
  second was an `Unsupported` "curved and B-spline surfaces", which reads as
  valid IFC this bridge declines rather than a broken reference. Applies to
  faces inside advanced B-reps too.
- Compiled `IfcBlock` meshes were offset by half their extents. `IfcBlock`
  has a corner at its `Position` (IFC4 ADD2 TC1), but the neutral
  `Primitive::Block` is tessellated centred on its origin by
  `axiolid-reference`, so every compiled block sat half its size away from
  where the file placed it, along its own axes. Lowering now puts the
  half-extent shift on the block's `Instance`. This changes compiled
  geometry for every `IfcBlock`; the lowered `Instance` translation now
  names the block's centre. The other CSG primitives already agreed.
- `Plane`, `CylindricalSurface`, `SphericalSurface` and `ToroidalSurface`
  `::position(&model)` now type-check their target through
  `resource::resolve` (#135). A `Position` naming anything other than an
  `IfcAxis2Placement3D` is `WrongEntityType` naming the target; it used to
  be wrapped as a 3D placement and misread.
- A derived linear placement no longer reports an evaluator's degenerate
  curve as an undefined roll. The refusal now distinguishes an unsupported
  curve family, a rejected measure (off the curve, or roll undefined
  because the tangent is parallel to the up reference) and a degenerate
  curve, matching how `axiolid-evaluate` 0.3 reports them.

- An opening that a file makes void two hosts is subtracted from the first
  only (#59). `IfcFeatureElementSubtraction.VoidsElements` is a
  single-valued inverse in IFC2X3 and IFC4, but every `IfcRelVoidsElement`
  was applied, so the second host's net body was cut by an opening that
  belongs to another element. The relation with the lower id now wins in
  `openings_of` and in net compilation. Output changes only for files that
  violate the schema.
- `IfcParameterizedProfileDef.Position` now reaches the kernel for every
  parameterised family (#147). It was applied to rectangles and circles
  only, so an I, L, T, U, C or Z section, an ellipse or a trapezium with an
  offset or rotated `Position` lowered at the profile origin. Output changes
  only for files that author a non-identity `Position` on those families.
- The translation of an `IfcDerivedProfileDef` operator is converted to
  metres (#147). It was passed through in file units, so a derived profile
  offset by 50 mm in a millimetre file lowered 50 m away.
- An `IfcAsymmetricIShapeProfileDef` in an IFC2X3 file no longer reads
  `CentreOfGravityInY` (slot 11 in that schema) as `BottomFlangeEdgeRadius`
  (#147). IFC2X3 declares the entity as an `IfcIShapeProfileDef` subtype
  with a different tail; the declared schema now selects the layout, and
  the IFC4-only edge radii and slopes are absent in IFC2X3.

### Added

- `IfcFaceSurface` and `IfcAdvancedFace` lower as representation items
  (#155). Both are members of `IfcSurfaceOrFaceSurface`, the type of a
  connection surface, but the dispatcher refused them. The new
  `lower_face_surface_node` builds ONE face through the B-rep face path --
  exact carrier surface, `SameSense` as the face's orientation relative to
  that carrier, each bound's `Orientation` -- inside one open shell with no
  solid, so a single face never acquires a volume. They move from the
  nested-only disposition ledger to `IMPLEMENTED`, and `BodyKind::Face`
  describes them. Committed evidence:
  `test/fixtures/synthetic-surfaces/synthetic_space_boundary_face_surface.ifc`.
- `lower_connection_surface(session, connection, frame)` and
  `lower_related_connection_surface` lower an
  `IfcConnectionSurfaceGeometry`'s `SurfaceOnRelatingElement` and optional
  `SurfaceOnRelatedElement` (#155), dispatching `IfcSurface`,
  `IfcFaceSurface`/`IfcAdvancedFace` and `IfcFaceBasedSurfaceModel`. Each
  end is authored in its own element's coordinate system, so each takes its
  own frame. Point, eccentric point, curve, volume and (IFC2X3) port
  connections are typed `Unsupported` refusals naming the connection; any
  other entity is `WrongEntityType`.
- `body_description(model, units, product)` reports how a product's Body
  representation is modelled (#147). It returns one `BodyItem` per
  geometric item in authored order, with mapped items resolved (`mapped_by`
  names the chain), each carrying a `BodyKind` (extrusion, tapered
  extrusion, revolution, tapered revolution, directrix sweep, swept disk,
  sectioned spine, B-rep, CSG, CSG primitive, half space, bounding box,
  tessellated, surface model, geometric set, curve, surface, point) and, for
  the swept-area families, a `SweptSolid`: the profile description, the
  end profile of a tapered sweep, the solid's placement in world
  coordinates and a `SweepPath` (extrusion direction as a world unit vector
  plus depth in metres, revolution axis and angle in radians, or the
  directrix curve). Mapped geometry describes identically to the same
  geometry authored in place. Anything that cannot be stated exactly is a
  typed error for the whole body, never a partial list: unsupported item or
  profile families, dangling references, mapping cycles, open profiles
  swept as areas, and a mapping that scales or mirrors a swept solid. It is
  kernel-free and reachable with `--no-default-features`.
- `describe_profile(model, units, profile)` reads any concrete
  `IfcProfileDef` into a `ProfileDescription`: type, `ProfileName`,
  `Position`, and `ProfileParameters` for every family in metres and
  radians (rectangle, rounded and hollow rectangle, circle and hollow
  circle, ellipse, I, asymmetric I, L, T, U, C, Z, trapezium, arbitrary
  closed, with voids and open by curve reference, centre line, composite,
  derived with its operator, mirrored). A bare `IfcProfileDef` and unknown
  families are refused; a composite or derived chain that references itself
  is `CyclicChain`.
- `derive_placement_transform` derives a linear placement on an
  `IfcPolyline` or a line-only `IfcIndexedPolyCurve` basis curve (#96), not
  only on an alignment. The curve lowers to the neutral polyline, whose arc
  length is an exact finite sum, so a distance converts to a parameter
  exactly; a native parameter follows the IFC polyline parameterisation
  (one per segment). Refused by name: a zero-length segment, fewer than two
  points, non-consecutive `Segments`, an `IfcArcIndex`, a parameter on a
  multi-point `IfcLineIndex` (IFC does not state its split), and ellipse
  and B-spline bases as before.
- `product_bounds` bounds linear extrusions of straight-edged profiles
  (rectangles, polyline contours, and placed/derived forms of them) and
  blocks exactly from their vertices, without tessellating (#98). The
  result reports `BoundsSource::Exact`. Curved profiles, rounded
  rectangles and booleans still go through the compiled mesh; a
  difference only shrinks its operand, so its operand's box is never used.
- `voiding_conflicts(model)` and `VoidingConflict { opening, kept_host,
  rejected_host, relation }` report such openings (#59). It is kernel-free,
  like `openings_of`. Restating the same host is not a conflict.
- Resolving typed accessors beside the raw `*_ref` getters (#97), following
  `Plane::position(&model)`: `Line::point`, `Polyline::points`,
  `IndexedPolyCurve::points`, `BSplineCurve::control_points`,
  `OffsetCurve3D::ref_direction`, `Circle::position`, `Ellipse::position`,
  `Trim::cartesian_point`, `BSplineSurface::control_point_views`,
  `SurfaceOfLinearExtrusion::{position, extruded_direction}`,
  `SurfaceOfRevolution::{position, axis_position}`,
  `BoundingBox::corner_point` and `TessellatedFaceSet::coordinate_list`.
  Each type-checks its target: a dangling reference is `MissingEntity`
  naming the referrer, a wrong type is `WrongEntityType` naming the target.
  All are kernel-free.
- `resource::resolve`, the shared type-checked resolvers behind them, and
  two select views: `Axis2Placement` (2D or 3D, for `IfcConic.Position`) and
  `CartesianPointList` (2D or 3D, for `IfcIndexedPolyCurve.Points`).
  Profiles stay references; `describe_profile` owns `IfcProfileDef` reading.

- `compile::product_bounds` / `product_bounds_with` return a product's
  world-space axis-aligned bounding box (#36). The body is resolved and placed
  the same way as for `compile_product_mesh`, and the result is `Ok(None)` in
  the same case (no body).
  - When every leaf of the lowered graph is a mesh or an authored bounding
    box, the box is read off the exact graph without tessellating
    (`BoundsSource::Exact`).
  - Otherwise it comes from the compiled mesh (`BoundsSource::Tessellated`).
    That box is exact for planar geometry and can fall short of a curved
    surface by up to the tolerance.
  - A body with no finite extent is `GeometryError::Degenerate`, never an
    empty box.
- `examples/product_bvh.rs` indexes every product of a file in
  `axiolid_spatial::Bvh` and prints the broad-phase overlaps.
  `axiolid-spatial` is a dev-dependency only: the index stays Axiolid's, and
  this crate only produces the boxes.

### Changed

- `lower::profile` builds every profile from `describe_profile` instead of
  reading slots itself (#147), so lowering and body description cannot
  disagree about a slot, a unit or a default. A profile nesting chain that
  exceeds its budget is now `ChainTooDeep` rather than `Unsupported`, and a
  self-referencing chain is `CyclicChain`; a dangling boundary curve of an
  arbitrary profile is reported when the profile is read.
- Requires `axiolid-mesh-compile` 0.3.4, `axiolid-construct` 0.3.3 and
  `axiolid-evaluate` 0.3.1. Compiled output changes where the kernel's did:
  a B-rep's void shells are tessellated facing into the cavity instead of
  being dropped, so an authored cavity is no longer filled
  (axiolid/kernel#120); every solid of a multi-solid B-rep is meshed, not
  only the first (axiolid/kernel#111); and a curve-bounded plane, the usual
  space-boundary connection surface, compiles to a planar surface mesh
  instead of being refused (axiolid/kernel#192).
- Requires `axiolid-mesh-compile` 0.3.3 and `axiolid-contracts` 0.3.1.
  Closed `IfcPolygonalFaceSet` bodies whose face corners lie on a straight
  run (collinear notch and window heads) now mesh closed and report `Solid`
  (axiolid/kernel#170); before, the triangulation left T-junction cracks and
  they came back `Surface`. Surface models with a zero-area bowtie face, as
  Nova MEP exports write pipe-fitting end caps, now compile instead of being
  refused (axiolid/kernel#171). On 12 real models (66,659 products) this
  moves 1,999 products to `Solid` and failures from 2,291 to 755, together
  with the kernel#168 and #169 fixes already required.
- `tests/meshing_coverage.rs` pins both: a real ArchiCAD lining at its
  exact coordinates meshes to its divergence volume as a solid, and a
  surface model with a bowtie cap keeps its area. A third test checks that
  an explicit chord budget (`ExecutionOptions::with_chord_error`,
  axiolid/kernel#165) brings the composite-curve D within 1e-5 of its exact
  volume.

## [0.3.1] - 2026-09-25

### Added

- `compile::compile_product_mesh_reported` (and `_with`) return a
  `CompiledMesh`: the triangles plus the kernel's `MeshClosure`, i.e. whether
  they bound a solid. `CompiledMesh::solid_mesh(product)` returns the mesh
  only for `Solid` and otherwise refuses with the new
  `GeometryError::NotASolid`, naming the product. An
  `IfcShellBasedSurfaceModel` now compiles (axiolid/kernel#161) but is a
  `Surface`: without this a caller summing the divergence of its triangles
  gets a volume the file never claimed, finite and plausible for a closed
  shell. A backend that does not report closure gives `Unknown`, which is
  refused as well. `NetMesh::closure` carries the flag for net bodies.
  `compile_product_mesh` is unchanged and still returns the bare mesh.
- `tests/meshing_coverage.rs` and the generated public fixture
  `test/fixtures/synthetic-coverage/meshing_coverage.ifc` (#47): one product
  for each product-meshing failure kind #47 measured on real models, compiled
  through `compile_product_mesh`. The four kinds fixed here (#43 to #46) pin
  exact volumes, cross-checked with IfcOpenShell 0.8.5. The two fixed in the
  Axiolid reference compiler (`axiolid-mesh-compile` 0.3.1) pin their answer
  too: polygonal faces with more than 3 corners (axiolid/kernel#160) mesh the
  2 m quad cube to 8 m3, and the open-shell surface model
  (axiolid/kernel#161) meshes its 0.12 m2 and refuses any volume.
- `DegenerateFacePolicy` (#46), set per session with
  `LoweringSession::with_face_policy`. The default, `Refuse`, is unchanged:
  an `IfcPolyLoop` with fewer than three distinct edges refuses the brep,
  naming the loop. `DropAndReport` leaves out a face whose outer (or only)
  bound collapses, since it covers no area, and lists it in
  `ProvenanceMap::dropped_faces`. A collapsed hole in a face with real area
  is still refused, and a shell whose every face collapses is refused as
  `Degenerate`. The policy drops exactly what the default refuses.
- On OfficeBuilding.ifc all 16 `IfcWindow`s refused this way (two shared
  loops of the form `(A, A, B, B)`) compile under `DropAndReport`, each
  reporting its one dropped face; their volume, 0.1707752 m3, matches
  IfcOpenShell 0.8.5. No other product in eight real models changes.
- `IfcArbitraryClosedProfileDef` and `IfcArbitraryProfileDefWithVoids` now
  lower an `IfcCompositeCurve` outer or inner boundary (#43). Segments may be
  `IfcPolyline`, `IfcTrimmedCurve` over `IfcCircle` or `IfcLine`, or a nested
  `IfcCompositeCurve`. Arcs stay exact `Circle2` segments. `SameSense`,
  `SenseAgreement` and `MasterRepresentation` are honoured, and a trim may be a
  parameter (in the project's plane-angle unit) or a cartesian point.
- On the two real models that carried them, all 128 products refused for a
  composite profile boundary now compile, and no other product changed.
- Net geometry (#44, ADR 0014): `compile::compile_product_mesh_net` (and
  `_with` for your own backend) returns a product's Body with every
  `IfcRelVoidsElement` opening subtracted, plus the ids of the openings
  removed. `compile_product_mesh` is unchanged and stays gross: quantity
  takeoff wants gross, clearance and ratio checks want net. The graph-level
  entry point is `lower::lower_product_net`; the kernel-free relation reader is
  `openings_of`.
- An opening that cannot be removed -- its Body does not lower, it has none,
  it is not a solid, or the backend refuses it or its cut -- is
  `GeometryError::OpeningNotSubtracted` naming the host and the opening. The
  gross body is never returned in its place. A host whose own Body the backend
  refuses stays `CompilationRefused` on the host.
- Multi-item hosts and openings (a Body with several items, or a mapped item)
  are cut item by item: every host part minus every opening part. A mapped
  opening is flattened through its instance transforms, composed outer after
  inner.

### Changed

- The workspace requires `axiolid-mesh-compile` 0.3.2 and
  `axiolid-mesh-compile-contract` 0.3.1. With 0.3.0 an `IfcPolygonalFaceSet`
  with any face of more than 3 corners, or with voids
  (`IfcIndexedPolygonalFaceWithVoids`), was refused as
  `Unsupported(Tessellation)`
  (axiolid/kernel#160), and every `IfcShellBasedSurfaceModel` as "brep has no
  solid" (axiolid/kernel#161). 0.3.1 triangulates such faces in their own
  plane and refuses a non-planar one by face index. With 0.3.1 a bend
  trimmed from a circle across its seam, as Revit writes a bar's bends
  (`270 -> 45` or `270 -> 15` degrees), was sampled around the wrong side of
  the circle, so the bend missed the next leg and the bar was refused as
  `composite directrix has a N unit gap` (axiolid/kernel#168). 0.3.2 runs the
  trim from its first end the way its sense says, wrapping past the seam. On
  the Revit rebar model below that compiles the last 1,494 refused bars: 40,990
  of 41,019 products compile, and no local model refuses a directrix gap.
- The workspace requires `axiolid-construct` 0.3.2 (reached through
  `axiolid-mesh-compile`, pinned only as a floor behind
  `compile-reference-backend`). With 0.3.0 two compiled results were wrong or
  refused, though lowering was right: an `IfcPolygonalBoundedHalfSpace` whose
  `Position` is translated within the clip plane cut the wrong region with no
  error (axiolid/kernel#164), and an opening body extruded downward, as
  Solibri and Revit hang windows from the lintel, was wound inside-out, so
  its subtraction was refused (axiolid/kernel#166; 78 of 423 real hosts
  refused with 0.3.0, 14 with 0.3.1). Both tests that pinned these now run.
  With 0.3.1 a swept disk was oriented by one fixed axis seeded from its
  first segment: a bent bar whose later leg ran along that axis was refused,
  and a leg nearly along it twisted the tube so its volume came out low with
  no error (axiolid/kernel#169). 0.3.2 carries the frame along the path. On
  the 41,019-product Revit rebar model below, the 878 refused bars compile
  and swept-disk bars within 0.5 % of their closed-form volume go from
  30,199 to 34,003 of 39,215; none is more than 1 % off.
- A gap between consecutive composite segments, or between the last and the
  first, wider than 1e-5 m is refused as `Degenerate`, naming the segment
  and the gap. It is never bridged with an edge the file did not author.
- Any other segment parent, a reparametrised segment, and a conic placed with
  a 3D placement stay typed `Unsupported`, naming the entity.

### Known limits

- The compiled mesh of a curved profile is only as close to the exact area as
  the kernel's chord budget allows. Up to `axiolid-mesh-compile` 0.3.2 that
  budget equals the linear tolerance, which is coarse for small radii: a real
  gutter profile meshes 1.9 % over its exact area and a slot 0.7 % under
  (axiolid/kernel#165). Lowering is exact; the arcs reach the kernel as arcs.
- On the local real-model corpus (423 hosts with openings whose gross Body
  compiles, six models) 14 hosts are still refused: the kernel boolean
  refuses a non-manifold operand, and each refusal names the opening. No
  host that already netted changed volume between kernels.
- Measured against IfcOpenShell 0.8.5 on 138 sampled hosts across four real
  models: 130 agree within 0.1 % (median difference about 1e-10). The other
  8 are not subtraction errors. On 7, IfcOpenShell closes a gap in the host's
  composite-curve profile with a segment the file never authored; our gross
  matches an independent exact integration of the profile to 1e-4 on the 4
  checked, IfcOpenShell's is off by up to 7 %. On 1, IfcOpenShell's own
  boolean fails and it returns the host uncut.

### Fixed

- `IfcPolygonalBoundedHalfSpace` clips now compile (#45). `PolygonalBoundary`
  was lowered through the 3D curve path as a `Curve3`, but Axiolid's
  `BoundedHalfSpace` contract and reference compiler require a `Curve2`, so
  every such `IfcBooleanClippingResult` was refused with `half-space boundary
  .. is not a Curve2 node` although lowering succeeded. The boundary now
  lowers as a `Curve2` polyline in `Position`'s XY plane, lengths converted
  to metres. A 3D boundary point is accepted only with `z = 0`; any other `z`
  violates `BoundaryDim` and is refused as `Degenerate`, naming the point,
  instead of being projected.
- `IfcSweptDiskSolid`, `IfcSweptDiskSolidPolygonal`,
  `IfcFixedReferenceSweptAreaSolid` and `IfcSurfaceCurveSweptAreaSolid` read
  `StartParam`/`EndParam` in their directrix's own parameterisation. On an
  `IfcCompositeCurve` that is not a length: ISO 10303-42 accumulates each
  segment's parametric length, 1 per `IfcPolyline` edge and a trimmed
  segment's own trim span, so an arc contributes its ANGLE in the file's
  plane-angle unit (IFC4 `IfcCompositeCurve`, figure 389: a line plus a 90
  degree arc is 91). It was converted as a length and handed to the kernel,
  which measures arc length, so a Revit rebar authored `(0, 365)` over five
  1-unit legs and four 90 degree bends was read as 365 m and refused, and a
  range that happened to fit silently cut the bar short. A full range now
  keeps the authored directrix; a partial range is cut exactly at the
  composite's parameters before the kernel sees it, honouring `SameSense`,
  `SenseAgreement`, `ParamLength` and arcs across the circle's seam. A range
  past the composite's parametric length is `Degenerate`, naming the sweep and
  the length; a range over a segment trimmed only by points is `Unsupported`,
  since its parametric length is not stated. A trimmed-curve directrix now
  converts by its basis, like any trim, instead of always as an angle.
- On the local real-model corpus (eight models) this changes one model: in a
  41,019-product Revit rebar model, compiled products go from 19,832 to
  38,618. Checked against the closed-form volume (inscribed disk polygon
  times exact path length) of all 39,215 swept-disk rebars: before, 25 were
  within 0.5 % and 18,077 compiled wrong, up to 96 % short; with this fix
  alone (`axiolid-construct` 0.3.1) 30,199 are within 0.5 %, and 34,003 with
  the kernel floors above. No product in any other model changed volume.

## [0.3.0] - 2026-09-23

### Added

- Lowering an `IfcTriangulatedFaceSet` now attaches texture coordinates from
  its `IfcIndexedTriangleTextureMap` as a corner-indexed attribute channel
  named `uv` (`lower::tessellated::UV_CHANNEL`), one `(s, t)` per triangle
  corner (#30). Positions stay shared, so the mesh stays closed. A shorter
  `TexCoordIndex` leaves trailing triangles `UNMAPPED`; an omitted one adds
  no channel; a longer one, or an index outside the texture vertices, is an
  error. A second map on the same face set becomes `uv1`, and so on.

### Known limits

- `IfcIndexedPolygonalTextureMap` (IFC4X3) is not lowered.

### Changed

- **Breaking:** requires Axiolid 0.3 (`axiolid-mesh` 0.3 adds
  `AttributeChannel::corner_indices`).

## [0.2.0] - 2026-09-22

First release under per-crate versioning. See the
[repository changelog](../../CHANGELOG.md) for the family-wide history
that produced this version.

[Unreleased]: https://github.com/openbimrs/ifc/compare/ifc-geometry-v0.21.0...HEAD
[0.21.0]: https://github.com/openbimrs/ifc/releases/tag/ifc-geometry-v0.21.0
[0.20.0]: https://github.com/openbimrs/ifc/releases/tag/ifc-geometry-v0.20.0
[0.19.0]: https://github.com/openbimrs/ifc/releases/tag/ifc-geometry-v0.19.0
[0.18.0]: https://github.com/openbimrs/ifc/releases/tag/ifc-geometry-v0.18.0
[0.17.0]: https://github.com/openbimrs/ifc/releases/tag/ifc-geometry-v0.17.0
[0.16.0]: https://github.com/openbimrs/ifc/releases/tag/ifc-geometry-v0.16.0
[0.15.0]: https://github.com/openbimrs/ifc/releases/tag/ifc-geometry-v0.15.0
[0.14.0]: https://github.com/openbimrs/ifc/releases/tag/ifc-geometry-v0.14.0
[0.13.0]: https://github.com/openbimrs/ifc/releases/tag/ifc-geometry-v0.13.0
[0.12.0]: https://github.com/openbimrs/ifc/releases/tag/ifc-geometry-v0.12.0
[0.11.0]: https://github.com/openbimrs/ifc/releases/tag/ifc-geometry-v0.11.0
[0.10.0]: https://github.com/openbimrs/ifc/releases/tag/ifc-geometry-v0.10.0
[0.9.0]: https://github.com/openbimrs/ifc/releases/tag/ifc-geometry-v0.9.0
[0.8.1]: https://github.com/openbimrs/ifc/releases/tag/ifc-geometry-v0.8.1
[0.8.0]: https://github.com/openbimrs/ifc/releases/tag/ifc-geometry-v0.8.0
[0.7.0]: https://github.com/openbimrs/ifc/releases/tag/ifc-geometry-v0.7.0
[0.6.1]: https://github.com/openbimrs/ifc/releases/tag/ifc-geometry-v0.6.1
[0.6.0]: https://github.com/openbimrs/ifc/releases/tag/ifc-geometry-v0.6.0
[0.5.0]: https://github.com/openbimrs/ifc/releases/tag/ifc-geometry-v0.5.0
[0.4.4]: https://github.com/openbimrs/ifc/releases/tag/ifc-geometry-v0.4.4
[0.4.3]: https://github.com/openbimrs/ifc/releases/tag/ifc-geometry-v0.4.3
[0.4.2]: https://github.com/openbimrs/ifc/releases/tag/ifc-geometry-v0.4.2
[0.4.1]: https://github.com/openbimrs/ifc/releases/tag/ifc-geometry-v0.4.1
[0.4.0]: https://github.com/openbimrs/ifc/releases/tag/ifc-geometry-v0.4.0
[0.3.1]: https://github.com/openbimrs/ifc/releases/tag/ifc-geometry-v0.3.1
[0.3.0]: https://github.com/openbimrs/ifc/releases/tag/ifc-geometry-v0.3.0
[0.2.0]: https://github.com/openbimrs/ifc/releases/tag/v0.2.0
