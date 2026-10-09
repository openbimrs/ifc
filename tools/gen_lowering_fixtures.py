#!/usr/bin/env python3
"""Generate the product-lowering fixtures for #335, #336, #346, #351, #353,
#354, #357, #362, #363, #388, #393, #396 and #398.

Thirteen small files, each one edge case of `crates/ifc-geometry`'s product
lowering. No licence-clean public corpus isolates these cases, so they are
generated; the output is our own data. Metres and radians, fixed GUIDs and
time stamp: regeneration is byte-stable.

`reference_view_openings.ifc` (IFC4, Reference View, #351). Four walls in a
storey, each an `IfcWall` whose `Body` is four tessellated boxes around a
1.0 x 2.0 m hole -- the hole an IFC4 Reference View export has already cut,
so the Body IS the net shape. Each wall is voided by:

- W1: an `IfcOpeningElement` with only a `Reference` representation, a
  tessellated box filling the hole. IFC4 says such a representation "is not
  subtracted, it is provided in addition to the hole in the Body".
- W2: an `IfcOpeningElement` with no representation at all.
- W3: the `Reference`-only opening of W1, and a second opening with a
  `Body` extrusion (x 3.0..3.5, z 1.0..2.0) that the tessellated Body does
  NOT carry, so it is to be subtracted.
- W4: an opening with a `Reference` AND a `Box` representation, which is not
  `Reference`-only.

`linear_placement_uncached.ifc` and `linear_placement_cache_check.ifc`
(IFC4X3_ADD2, #353 and #354) share one alignment centreline, an
`IfcGradientCurve`: plan, a 100 m line along +X from the origin; profile, a
constant grade of 0.02 from 10 m. Every product is an
`IfcBuildingElementProxy` with a 1 m `IfcBlock` Body centred on its local
origin in plan and standing on it, placed by an `IfcLinearPlacement` whose
`IfcPointByDistanceExpression` names the `IfcGradientCurve` directly.

At 40 m (plan distance) the centreline is (40, 0, 10.8) with tangent
(1, 0, 0.02) / sqrt(1.0004); IFC4.3 puts a positive `OffsetLateral` to the
left, (0, 1, 0), so 40 m with a lateral offset of 2 m is (40, 2, 10.8).

- uncached: `LENGTH` (40 m, 2 m left, no `CartesianPosition`) and `AXES`
  (40 m, 2 m left, with explicit `Axis` (0, 0, 1) and `RefDirection`
  (0, 1, 0) in the curve's (tangent, left, up) frame). The test turns
  `LENGTH`'s measure into an `IfcParameterValue` in memory for the #347
  refusal: every item in a committed fixture must lower
  (`tests/lower_dispatch_corpus.rs`).
- cache check (declared `Precision` 1e-5 m): the same 40 m / 2 m left
  expression with a cached `CartesianPosition` at `MATCH` (40, 2, 10.8),
  `NEAR` 4e-6 m off in X (inside the precision), `OUTSIDE` 5e-5 m off in X
  (outside it) and `STALE` (60, 2, 11.2), 20 m off -- where the product sat
  before its alignment was edited, say.

`linear_placement_alignment_frame.ifc` (IFC4X3_ADD2, #357, #363). The same
centreline, carried by an `IfcAlignment` placed OFF identity: the site sits
at (100, 0, 0), the alignment at (400, 300, 20) relative to it, turned a
quarter left (its x along world +Y, its y along world -X). The context's
`WorldCoordinateSystem` is a translation by (10, 20, 0). The station 40 m
along, 2 m left, (40, 2, 10.8) in the alignment's frame, is therefore
(498, 340, 30.8) in placement space and (508, 360, 30.8) after the WCS.

- `REL_TO`: `PlacementRelTo` the alignment's placement, no cache.
- `IMPLIED`: no `PlacementRelTo` (IFC4.3: then "established by the origin
  of horizontal alignment of the referenced IfcAlignment Axis"), no cache.
- `CACHED`: `PlacementRelTo` the alignment's placement, with the correct
  `CartesianPosition` (40, 2, 10.8) relative to it.
- `CACHED_IMPLIED`: no `PlacementRelTo`, the same cache.
- `BRACKET_CACHED` and `BRACKET_DERIVED`: `IfcLocalPlacement`s at (1, 0, 3)
  relative to the linear placements of `CACHED` and `REL_TO` (#363).

`grid_placement.ifc` (IFC4) and `grid_placement_ifc4x3.ifc` (IFC4X3_ADD2,
whose `IfcGridPlacement` carries the inherited `PlacementRelTo`, set to the
grid's placement) state one grid (#362, #363). The storey is at (0, 0, 3);
the grid at (10, 5, 0) relative to it, its x along (0.6, 0.8, 0). UAxes:
`A` an `IfcPolyline` x = 0 running +y; `B` an `IfcPolyline` x = 6 running
+y with `SameSense` FALSE; `R` an `IfcTrimmedCurve` on an `IfcCircle` of
radius 10 about the origin, 0 to pi/2. VAxes: `1` an `IfcLine` y = 0
running +x; `2` an `IfcTrimmedCurve` on an `IfcLine` y = 8; `3` an
`IfcOffsetCurve2D` of axis 1's line by 5 (y = 5). Products, in the grid's
frame:

- `A1_OFFSET`: A/1 with offsets (0.5, -0.25, 1.0): the offset curves are
  x = -0.5 (left of +y) and y = -0.25, so (-0.5, -0.25, 1.0), x along A's
  tangent (0, 1).
- `B2_DIRECTION`: B/2, `PlacementRefDirection` the `IfcDirection`
  (1, 1, 5): (6, 8, 0), x along (1, 1) / sqrt(2) (x and y ratios only).
- `A1_TOWARDS_B2`: A/1, `PlacementRefDirection` the intersection B/2:
  (0, 0, 0), x along (0.6, 0.8).
- `B1_SAMESENSE`: B/1 with offsets (0.5, 0): B runs -y, so its left is +x
  and the offset curve is x = 6.5: (6.5, 0, 0), x along (0, -1).
- `R3_CURVED`: R/3 with offsets (0.5, 0): R runs anticlockwise, its left
  is inward, so the offset curve has radius 9.5, and meets y = 5 at
  (sqrt(65.25), 5).
- `BRACKET`: an `IfcLocalPlacement` at (1, 0, 2) relative to
  `A1_OFFSET`'s grid placement (#363).

`indexed_profile_boundaries.ifc` (IFC4) and
`indexed_profile_boundaries_ifc4x3.ifc` (IFC4X3_ADD2, whose arc point lists
carry a `TagList`) state arbitrary profiles bounded by `IfcIndexedPolyCurve`s
(#335), each next to the `IfcPolyline` or `IfcCompositeCurve` that states the
same outline. Every profile is extruded 1 m by its own proxy, named as the
profile, so a compiled volume is the profile's area:

- `INDEXED_D`: the square [-1, 1]^2 whose right edge is the half circle
  through (1, -1), (2, 0), (1, 1): `IfcLineIndex`, `IfcArcIndex`,
  `IfcLineIndex`, anticlockwise. Area 4 + pi/2.
- `COMPOSITE_D`: the same as an `IfcCompositeCurve`: two polylines and an
  `IfcTrimmedCurve` on an `IfcCircle` about (1, 0), trimmed at Cartesian
  points.
- `INDEXED_D_CLOCKWISE`, `COMPOSITE_D_CLOCKWISE`: the same, clockwise; the
  composite's arc has `SenseAgreement` FALSE.
- `INDEXED_POLYLINE`: a 2 x 1 rectangle, no `Segments`, closed by repeating
  its first point. `POLYLINE`: the same as an `IfcPolyline`.
- `INDEXED_WITH_VOIDS`: an `IfcArbitraryProfileDefWithVoids`: the 4 x 4
  square about the origin as one `IfcLineIndex`, and a unit circle of two
  `IfcArcIndex` segments as its void. Area 16 - pi.

`curve_bounded_plane_composite.ifc` (IFC4, #336). Two `IfcCurveBoundedPlane`
bodies whose boundaries are `IfcCompositeCurve`s of `IfcPolyline` segments:

- `SEGMENTS_WITH_HOLE`: a 4 x 3 outer boundary of three segments, the middle
  one authored backwards with `SameSense` FALSE, and a 1 x 1 hole of two
  segments at (1, 1). Area 11.
- `ONE_REVERSED_SEGMENT`: one segment, `SameSense` FALSE, wrapping a
  clockwise closed polyline, on a plane at z = 5. Area 12.

`flush_openings_site_placements.ifc` (IFC4, #388). One wall under four
`IfcSite`s, named as the site: `ORIGIN`, `ORIGIN_TURNED`, `GEOREF` and
`GEOREF_TURNED`, placed at the origin or at (600 000, 5 600 000, 200), and
turned by `RefDirection` (0.999215271103513, 0.0396086100934191, 0) (about
2.3 degrees) or not. The wall is placed at (13.475, -15.95, 4.3) in its site,
turned 90 degrees; its Body is an `IfcRectangleProfileDef` 3.65 x 0.25 at
(1.825, 0), `RefDirection` (-1, 0), extruded 3.67. Two `IfcOpeningElement`s
are placed relative to the wall at (3.325, 0.125, 0.2) and (1.35, 0.125, 0.2):
a 1.01 x 2.26 rectangle at (1.13, 0.505), `RefDirection` (0, 1), in a
position with `Axis` (0, -1, 0) and `RefDirection` (0, 0, 1), extruded 0.25,
through the wall's whole thickness and flush with both faces. Net volume
3.65 * 0.25 * 3.67 - 2 * 1.01 * 2.26 * 0.25. Three more walls:

- `SIBLING` (in `GEOREF_TURNED`): the openings placed relative to the
  site, at the same position, so the placement the two share is the site's.
- `TWO_PART` (in `GEOREF_TURNED`): the wall as two 1.825 m solids meeting
  between the openings.
- `ABSOLUTE` (in `ORIGIN`): the openings placed with no `PlacementRelTo`,
  sharing no placement with the wall.

`halfspace_boundaries_ifc4x3.ifc` (IFC4X3_ADD2, #393, #398). Walls named as
their case, each a 4 x 1 x 3 extrusion over [0, 4] x [0, 1] clipped by an
`IfcPolygonalBoundedHalfSpace`: the plane z = 2, `AgreementFlag` FALSE (the
material above it is cut), `Position` the identity, and a boundary. The
first six state the anticlockwise pentagon (1, -1), (3, -1), (3, 0.5),
(2, 2), (1, 0.5), and the wall keeps 12 - 11/6 = 61/6. Boundaries:

- `POLYLINE`: the pentagon as an `IfcPolyline`, closed by repeating its
  first point's reference: the twin every other wall must equal.
- `COMPOSITE`: an `IfcCompositeCurve` of three polylines, the middle one
  (the gable's two edges) authored backwards with `SameSense` FALSE.
- `COMPOSITE_LINE`: an open polyline through the five points, closed by an
  `IfcTrimmedCurve` on an `IfcLine`.
- `INDEXED`: an `IfcIndexedPolyCurve` without `Segments`, closed by
  repeating its first point.
- `INDEXED_SEGMENTS`: an `IfcIndexedPolyCurve` with the `IfcLineIndex`
  segments (1, 2, 3), (3, 4) and (4, 5, 1).
- `INDEXED_COLLINEAR_ARC` (#396): the same segments with (3, 4) an
  `IfcArcIndex` (3, 6, 4) through a sixth point (2.5, 1.25), the gable
  edge's midpoint: collinear, so "treated as a polyline segment".

Two more walls have boundaries with genuine circular arcs (#398), which
lower to an exact profile contour, and have no polyline twin:

- `COMPOSITE_ARCS`: six `IfcCompositeCurveSegment`s, four two-point
  polylines and two `IfcTrimmedCurve`s on one `IfcCircle` of radius 1.2
  about (1, 0.5): the rectangle [1, 3.5] x [-0.22, 2.5] less that disk,
  the arc split at (2.2, 0.5). The upper arc is trimmed by parameter 0 to
  pi/2 and walked backwards (`SameSense` FALSE); the lower is trimmed by
  Cartesian points (2.2, 0.5) to (1.96, -0.22) with `SenseAgreement`
  FALSE. The wall keeps 12 - (2.5 - S), S the disk's area within the
  wall's strip 0 <= y <= 1 (`integral of sqrt(1.44 - u^2)` over
  |u| <= 0.5).
- `INDEXED_ARC`: an `IfcIndexedPolyCurve` over (1, -1), (3, -1), (3, 0.5),
  (2, 1.5), (1, 0.5) with segments `IfcLineIndex` (1, 2, 3), `IfcArcIndex`
  (3, 4, 5) and `IfcLineIndex` (5, 1): the rectangle under a half circle of
  radius 1 about (2, 0.5). The wall keeps 12 - (1 + sqrt(3)/4 + pi/6).

`indexed_curve_arcs.ifc` (IFC4, #396). Proxies whose `Axis` representation
(`Curve3D`) is an `IfcIndexedPolyCurve` over an `IfcCartesianPointList3D`,
named as their case:

- `COLLINEAR_ARC`: (0,0,0) -> (2,0,0) as an `IfcLineIndex`, the
  `IfcArcIndex` (2,0,0), (3,0,0), (4,0,0) -- its middle point between the
  others -- and (4,0,0) -> (4,2,0).
- `OUT_AND_BACK_ARC`: the `IfcArcIndex` (0,0,1), (4,0,1), (2,0,1), whose
  middle point lies beyond its end, then (2,0,1) -> (2,2,1).
- `NEAR_COLLINEAR_ARC`: the `IfcArcIndex` (0,0,2), (1,4e-6,2), (2,0,2),
  its middle point 4e-6 m off the chord, inside the declared 1e-5 m
  `Precision`.
- `ARC`: the genuine half circle (0,0,3), (1,1,3), (2,0,3).

`station_seams_ifc4x3.ifc` (IFC4X3_ADD2, #346, declared `Precision`
1e-5 m). Stations on the tangent discontinuities of two basis curves:

- an `IfcGradientCurve` over a 100 m plan line along +X, its profile a
  grade of 0.02 from height 10 for 40 m, then -0.01 for 60 m, with no
  vertical curve between: a grade break at 40 m (height 10.8);
- an L-shaped 3D `IfcPolyline` (0,0,0) -> (10,0,0) -> (10,10,0), a left
  corner at 10 m.

Products, each an `IfcBuildingElementProxy` at the origin:

- `GRADE_BREAK_AT`: a `Reference` point 40 m along the gradient curve,
  2 m `OffsetVertical`; `GRADE_BREAK_NEAR`: the same at 40.000004 m,
  inside the precision. IFC4.3 ADD2 8.9.3.48.3: the previous segment's
  tangent governs, so the offset is perpendicular to the 0.02 grade.
- `CORNER_AT`: a point 10 m along the L, 1 m left and 0.5 m
  `OffsetLongitudinal`: (10.5, 1, 0) on the incoming leg;
  `CORNER_NEAR`: the same at 9.999996 m.
- `DECK`: an `IfcSectionedSolidHorizontal` along the L, a 2 x 1 m
  rectangle at 5 m and at 15 m, across the corner: mitred at half angle
  (8.8.3.35.1), volume 2 x 1 x 10 = 20 m3, the mitre's corners at
  (11, -1) and (9, 1).
- `KERB`: an `IfcOffsetCurveByDistances` along the L, 1 m left at 2 m and
  18 m, carried to both ends (8.9.3.42.3), its `Axis`; and a 0.1 m
  `IfcSweptDiskSolid` along it as `Body`. Mitred, the offset runs
  (0,1) -> (9,1) -> (9,10).
- `CARRIAGEWAY`: an `IfcSectionedSurface` along the gradient curve, a
  flat open section 3.5 m each side at 30 m and at 50 m, across the grade
  break: mitred in the vertical plane, two planar strips of
  7 x 10 sqrt(1.0004) and 7 x 10 sqrt(1.0001) m2 meeting at
  (40, +-3.5, 10.8).

Run:  python3 tools/gen_lowering_fixtures.py test/fixtures/synthetic-lowering
"""

import math
import pathlib
import sys
import uuid

import ifcopenshell
import ifcopenshell.guid


def guid(label):
    """A GUID that is stable across regenerations."""
    return ifcopenshell.guid.compress(
        uuid.uuid5(uuid.NAMESPACE_URL, "openbimrs/ifc#351/lowering/" + label).hex)


def point(f, xyz):
    return f.create_entity("IfcCartesianPoint", Coordinates=[float(v) for v in xyz])


def direction(f, ratios):
    return f.create_entity("IfcDirection", DirectionRatios=[float(v) for v in ratios])


def place3(f, origin=(0.0, 0.0, 0.0), axis=None, ref=None):
    return f.create_entity(
        "IfcAxis2Placement3D", Location=point(f, origin),
        Axis=direction(f, axis) if axis else None,
        RefDirection=direction(f, ref) if ref else None)


def local(f, origin=(0.0, 0.0, 0.0), parent=None):
    return f.create_entity("IfcLocalPlacement", PlacementRelTo=parent,
                           RelativePlacement=place3(f, origin))


def units(f):
    metre = f.create_entity("IfcSIUnit", UnitType="LENGTHUNIT", Name="METRE")
    radian = f.create_entity("IfcSIUnit", UnitType="PLANEANGLEUNIT", Name="RADIAN")
    return f.create_entity("IfcUnitAssignment", Units=[metre, radian])


def context(f, precision):
    return f.create_entity(
        "IfcGeometricRepresentationContext", ContextType="Model",
        CoordinateSpaceDimension=3, Precision=precision,
        WorldCoordinateSystem=place3(f))


def rep(f, ctx, identifier, kind, items):
    return f.create_entity("IfcShapeRepresentation", ContextOfItems=ctx,
                           RepresentationIdentifier=identifier,
                           RepresentationType=kind, Items=items)


def shape(f, representations):
    return f.create_entity("IfcProductDefinitionShape", Representations=representations)


# Outward, counter-clockwise from outside; 1-based into the box corners
# (x0,y0,z0) (x1,y0,z0) (x1,y1,z0) (x0,y1,z0) and the same at z1.
BOX_TRIANGLES = [
    (1, 3, 2), (1, 4, 3),  # bottom
    (5, 6, 7), (5, 7, 8),  # top
    (1, 2, 6), (1, 6, 5),  # y0
    (4, 8, 7), (4, 7, 3),  # y1
    (1, 5, 8), (1, 8, 4),  # x0
    (2, 3, 7), (2, 7, 6),  # x1
]


def box(f, lo, hi):
    """A closed tessellated box between corners `lo` and `hi`."""
    (x0, y0, z0), (x1, y1, z1) = lo, hi
    corners = [(x0, y0, z0), (x1, y0, z0), (x1, y1, z0), (x0, y1, z0),
               (x0, y0, z1), (x1, y0, z1), (x1, y1, z1), (x0, y1, z1)]
    coordinates = f.create_entity(
        "IfcCartesianPointList3D", CoordList=[[float(v) for v in c] for c in corners])
    return f.create_entity("IfcTriangulatedFaceSet", Coordinates=coordinates,
                           Closed=True, CoordIndex=[list(t) for t in BOX_TRIANGLES])


# The wall: 4.0 long (x), 0.2 thick (y), 3.0 high (z); the hole the export
# has already cut is x 1.5..2.5, z 0.5..2.5.
WALL = ((0.0, 0.0, 0.0), (4.0, 0.2, 3.0))
HOLE = ((1.5, 0.0, 0.5), (2.5, 0.2, 2.5))


def voided_wall_pieces(f):
    """The wall around HOLE, as four tessellated boxes."""
    return [
        box(f, (0.0, 0.0, 0.0), (1.5, 0.2, 3.0)),  # left pier
        box(f, (2.5, 0.0, 0.0), (4.0, 0.2, 3.0)),  # right pier
        box(f, (1.5, 0.0, 0.0), (2.5, 0.2, 0.5)),  # sill
        box(f, (1.5, 0.0, 2.5), (2.5, 0.2, 3.0)),  # lintel
    ]


def reference_view():
    f = ifcopenshell.file(schema="IFC4")
    f.header.file_description.description = ("ViewDefinition [ReferenceView_V1.2]",)
    ctx = context(f, 1e-5)
    body_ctx = f.create_entity(
        "IfcGeometricRepresentationSubContext", ContextIdentifier="Body",
        ContextType="Model", ParentContext=ctx, TargetView="MODEL_VIEW")
    reference_ctx = f.create_entity(
        "IfcGeometricRepresentationSubContext", ContextIdentifier="Reference",
        ContextType="Model", ParentContext=ctx, TargetView="MODEL_VIEW")
    box_ctx = f.create_entity(
        "IfcGeometricRepresentationSubContext", ContextIdentifier="Box",
        ContextType="Model", ParentContext=ctx, TargetView="MODEL_VIEW")

    project = f.create_entity("IfcProject", GlobalId=guid("rv/project"),
                              Name="Reference View openings", UnitsInContext=units(f),
                              RepresentationContexts=[ctx])
    site_placement = local(f)
    site = f.create_entity("IfcSite", GlobalId=guid("rv/site"), Name="Site",
                           ObjectPlacement=site_placement)
    building_placement = local(f, parent=site_placement)
    building = f.create_entity("IfcBuilding", GlobalId=guid("rv/building"),
                               Name="Building", ObjectPlacement=building_placement)
    storey_placement = local(f, parent=building_placement)
    storey = f.create_entity("IfcBuildingStorey", GlobalId=guid("rv/storey"),
                             Name="Storey", ObjectPlacement=storey_placement,
                             Elevation=0.0)
    f.create_entity("IfcRelAggregates", GlobalId=guid("rv/agg-site"),
                    RelatingObject=project, RelatedObjects=[site])
    f.create_entity("IfcRelAggregates", GlobalId=guid("rv/agg-building"),
                    RelatingObject=site, RelatedObjects=[building])
    f.create_entity("IfcRelAggregates", GlobalId=guid("rv/agg-storey"),
                    RelatingObject=building, RelatedObjects=[storey])

    walls = []

    def wall(name, y):
        placement = local(f, (0.0, y, 0.0), storey_placement)
        element = f.create_entity(
            "IfcWall", GlobalId=guid("rv/" + name), Name=name,
            ObjectPlacement=placement, PredefinedType="NOTDEFINED",
            Representation=shape(f, [rep(f, body_ctx, "Body", "Tessellation",
                                         voided_wall_pieces(f))]))
        walls.append(element)
        return element, placement

    def opening(name, host, host_placement, representations):
        element = f.create_entity(
            "IfcOpeningElement", GlobalId=guid("rv/" + name), Name=name,
            ObjectPlacement=local(f, parent=host_placement), PredefinedType="OPENING",
            Representation=shape(f, representations) if representations else None)
        f.create_entity("IfcRelVoidsElement", GlobalId=guid("rv/voids-" + name),
                        RelatingBuildingElement=host, RelatedOpeningElement=element)
        return element

    def reference_only():
        return [rep(f, reference_ctx, "Reference", "Tessellation", [box(f, *HOLE)])]

    w1, w1_placement = wall("W1", 0.0)
    opening("O1-reference-only", w1, w1_placement, reference_only())

    w2, w2_placement = wall("W2", 5.0)
    opening("O2-no-representation", w2, w2_placement, None)

    w3, w3_placement = wall("W3", 10.0)
    opening("O3a-reference-only", w3, w3_placement, reference_only())
    profile = f.create_entity(
        "IfcRectangleProfileDef", ProfileType="AREA", XDim=0.5, YDim=1.0,
        Position=f.create_entity("IfcAxis2Placement2D", Location=f.create_entity(
            "IfcCartesianPoint", Coordinates=[3.25, 1.5])))
    # Through the wall's thickness: local z = -Y from y = 0.2, local x = +X,
    # so local y = z x x = +Z and the profile's (x, y) are the wall's (x, z).
    cutter = f.create_entity(
        "IfcExtrudedAreaSolid", SweptArea=profile,
        Position=place3(f, (0.0, 0.2, 0.0), axis=(0.0, -1.0, 0.0), ref=(1.0, 0.0, 0.0)),
        ExtrudedDirection=direction(f, (0.0, 0.0, 1.0)), Depth=0.2)
    opening("O3b-body", w3, w3_placement,
            [rep(f, body_ctx, "Body", "SweptSolid", [cutter])])

    w4, w4_placement = wall("W4", 15.0)
    bounding = f.create_entity("IfcBoundingBox", Corner=point(f, HOLE[0]),
                               XDim=1.0, YDim=0.2, ZDim=2.0)
    opening("O4-reference-and-box", w4, w4_placement,
            reference_only() + [rep(f, box_ctx, "Box", "BoundingBox", [bounding])])

    f.create_entity("IfcRelContainedInSpatialStructure", GlobalId=guid("rv/contained"),
                    RelatingStructure=storey, RelatedElements=walls)
    return f


GRADE = 0.02
START_HEIGHT = 10.0
PLAN_LENGTH = 100.0


def length(f, value):
    return f.create_entity("IfcLengthMeasure", float(value))


def line2(f):
    return f.create_entity(
        "IfcLine", Pnt=f.create_entity("IfcCartesianPoint", Coordinates=[0.0, 0.0]),
        Dir=f.create_entity("IfcVector", Orientation=direction(f, (1.0, 0.0)),
                            Magnitude=1.0))


def segment(f, origin, heading, run, parent, transition="CONTSAMEGRADIENT"):
    placement = f.create_entity(
        "IfcAxis2Placement2D",
        Location=f.create_entity("IfcCartesianPoint", Coordinates=[float(v) for v in origin]),
        RefDirection=direction(f, heading))
    return f.create_entity("IfcCurveSegment", Transition=transition, Placement=placement,
                           SegmentStart=length(f, 0.0), SegmentLength=length(f, run),
                           ParentCurve=parent)


def centreline(f):
    """The `IfcGradientCurve`, written the way IfcOpenShell's alignment API
    maps it: a profile segment's length is measured along the profile."""
    line = line2(f)
    plan = f.create_entity("IfcCompositeCurve", Segments=[
        segment(f, (0.0, 0.0), (1.0, 0.0), PLAN_LENGTH, line),
        segment(f, (PLAN_LENGTH, 0.0), (1.0, 0.0), 0.0, line, "DISCONTINUOUS"),
    ], SelfIntersect=False)
    norm = math.hypot(1.0, GRADE)
    heading = (1.0 / norm, GRADE / norm)
    end_height = START_HEIGHT + GRADE * PLAN_LENGTH
    profile = [
        segment(f, (0.0, START_HEIGHT), heading, PLAN_LENGTH * norm, line),
        segment(f, (PLAN_LENGTH, end_height), heading, 0.0, line, "DISCONTINUOUS"),
    ]
    return plan, f.create_entity("IfcGradientCurve", Segments=profile,
                                 SelfIntersect=False, BaseCurve=plan)


def along(f, basis, measure, lateral=None, axis=None, ref=None, cached=None, rel_to=None):
    expression = f.create_entity(
        "IfcPointByDistanceExpression", DistanceAlong=measure, OffsetLateral=lateral,
        BasisCurve=basis)
    relative = f.create_entity(
        "IfcAxis2PlacementLinear", Location=expression,
        Axis=direction(f, axis) if axis else None,
        RefDirection=direction(f, ref) if ref else None)
    return f.create_entity("IfcLinearPlacement", PlacementRelTo=rel_to,
                           RelativePlacement=relative,
                           CartesianPosition=place3(f, cached) if cached else None)


def block(f):
    """A 1 m block centred on its local origin in plan, standing on it."""
    return f.create_entity("IfcBlock", Position=place3(f, (-0.5, -0.5, 0.0)),
                           XLength=1.0, YLength=1.0, ZLength=1.0)


def proxy(f, label, name, placement, body_ctx):
    return f.create_entity(
        "IfcBuildingElementProxy", GlobalId=guid(label + "/" + name), Name=name,
        ObjectPlacement=placement,
        Representation=shape(f, [rep(f, body_ctx, "Body", "CSG", [block(f)])]))


def alignment_frame():
    """#357, #363: linear placements on an alignment placed off identity."""
    label = "alignment-frame"
    f = ifcopenshell.file(schema="IFC4X3_ADD2")
    ctx = f.create_entity(
        "IfcGeometricRepresentationContext", ContextType="Model",
        CoordinateSpaceDimension=3, Precision=1e-5,
        WorldCoordinateSystem=place3(f, (10.0, 20.0, 0.0)))
    body_ctx = f.create_entity(
        "IfcGeometricRepresentationSubContext", ContextIdentifier="Body",
        ContextType="Model", ParentContext=ctx, TargetView="MODEL_VIEW")
    project = f.create_entity("IfcProject", GlobalId=guid(label + "/project"),
                              Name=label, UnitsInContext=units(f),
                              RepresentationContexts=[ctx])
    site_placement = local(f, (100.0, 0.0, 0.0))
    site = f.create_entity("IfcSite", GlobalId=guid(label + "/site"), Name="Site",
                           ObjectPlacement=site_placement)
    plan, gradient = centreline(f)
    alignment_placement = f.create_entity(
        "IfcLocalPlacement", PlacementRelTo=site_placement,
        RelativePlacement=place3(f, (400.0, 300.0, 20.0), axis=(0.0, 0.0, 1.0),
                                 ref=(0.0, 1.0, 0.0)))
    alignment = f.create_entity(
        "IfcAlignment", GlobalId=guid(label + "/alignment"), Name="Alignment",
        ObjectPlacement=alignment_placement,
        Representation=shape(f, [rep(f, ctx, "FootPrint", "Curve2D", [plan]),
                                 rep(f, ctx, "Axis", "Curve3D", [gradient])]))

    def station(rel_to=None, cached=None):
        return along(f, gradient, length(f, 40.0), 2.0, cached=cached, rel_to=rel_to)

    rel_to = station(rel_to=alignment_placement)
    cached = station(rel_to=alignment_placement, cached=(40.0, 2.0, 10.8))
    contained = [
        proxy(f, label, "REL_TO", rel_to, body_ctx),
        proxy(f, label, "IMPLIED", station(), body_ctx),
        proxy(f, label, "CACHED", cached, body_ctx),
        proxy(f, label, "CACHED_IMPLIED", station(cached=(40.0, 2.0, 10.8)), body_ctx),
        proxy(f, label, "BRACKET_CACHED", local(f, (1.0, 0.0, 3.0), cached), body_ctx),
        proxy(f, label, "BRACKET_DERIVED", local(f, (1.0, 0.0, 3.0), rel_to), body_ctx),
    ]
    f.create_entity("IfcRelAggregates", GlobalId=guid(label + "/aggregates"),
                    RelatingObject=project, RelatedObjects=[site])
    f.create_entity("IfcRelAggregates", GlobalId=guid(label + "/alignment-aggregates"),
                    RelatingObject=project, RelatedObjects=[alignment])
    f.create_entity("IfcRelContainedInSpatialStructure", GlobalId=guid(label + "/contained"),
                    RelatingStructure=site, RelatedElements=contained)
    return f


def point2(f, xy):
    return f.create_entity("IfcCartesianPoint", Coordinates=[float(v) for v in xy])


def line_through(f, origin, heading):
    return f.create_entity(
        "IfcLine", Pnt=point2(f, origin),
        Dir=f.create_entity("IfcVector", Orientation=direction(f, heading), Magnitude=1.0))


def grid_file(schema):
    """#362, #363: one grid with straight, reversed, offset and curved axes."""
    label = "grid-" + schema
    f = ifcopenshell.file(schema=schema)
    ctx = context(f, 1e-5)
    body_ctx = f.create_entity(
        "IfcGeometricRepresentationSubContext", ContextIdentifier="Body",
        ContextType="Model", ParentContext=ctx, TargetView="MODEL_VIEW")
    project = f.create_entity("IfcProject", GlobalId=guid(label + "/project"),
                              Name=label, UnitsInContext=units(f),
                              RepresentationContexts=[ctx])
    site_placement = local(f)
    site = f.create_entity("IfcSite", GlobalId=guid(label + "/site"), Name="Site",
                           ObjectPlacement=site_placement)
    building_placement = local(f, parent=site_placement)
    building = f.create_entity("IfcBuilding", GlobalId=guid(label + "/building"),
                               Name="Building", ObjectPlacement=building_placement)
    storey_placement = local(f, (0.0, 0.0, 3.0), building_placement)
    storey = f.create_entity("IfcBuildingStorey", GlobalId=guid(label + "/storey"),
                             Name="Storey", ObjectPlacement=storey_placement,
                             Elevation=3.0)
    for parent, child in [(project, site), (site, building), (building, storey)]:
        f.create_entity("IfcRelAggregates", GlobalId=guid(label + "/agg-" + child.Name),
                        RelatingObject=parent, RelatedObjects=[child])

    def axis(tag, curve, same_sense=True):
        return f.create_entity("IfcGridAxis", AxisTag=tag, AxisCurve=curve,
                               SameSense=same_sense)

    def polyline(a, b):
        return f.create_entity("IfcPolyline", Points=[point2(f, a), point2(f, b)])

    def trimmed(basis, t1, t2):
        return f.create_entity(
            "IfcTrimmedCurve", BasisCurve=basis,
            Trim1=[f.create_entity("IfcParameterValue", float(t1))],
            Trim2=[f.create_entity("IfcParameterValue", float(t2))],
            SenseAgreement=True, MasterRepresentation="PARAMETER")

    line_1 = line_through(f, (0.0, 0.0), (1.0, 0.0))
    circle = f.create_entity(
        "IfcCircle", Position=f.create_entity("IfcAxis2Placement2D",
                                              Location=point2(f, (0.0, 0.0))),
        Radius=10.0)
    a = axis("A", polyline((0.0, 0.0), (0.0, 20.0)))
    b = axis("B", polyline((6.0, 0.0), (6.0, 20.0)), same_sense=False)
    r = axis("R", trimmed(circle, 0.0, math.pi / 2.0))
    one = axis("1", line_1)
    two = axis("2", trimmed(line_through(f, (0.0, 8.0), (1.0, 0.0)), 0.0, 20.0))
    three = axis("3", f.create_entity("IfcOffsetCurve2D", BasisCurve=line_1, Distance=5.0,
                                      SelfIntersect=False))
    grid_placement = f.create_entity(
        "IfcLocalPlacement", PlacementRelTo=storey_placement,
        RelativePlacement=place3(f, (10.0, 5.0, 0.0), axis=(0.0, 0.0, 1.0),
                                 ref=(0.6, 0.8, 0.0)))
    grid = f.create_entity("IfcGrid", GlobalId=guid(label + "/grid"), Name="Grid",
                           ObjectPlacement=grid_placement, UAxes=[a, b, r],
                           VAxes=[one, two, three])

    def at(first, second, offsets):
        return f.create_entity("IfcVirtualGridIntersection",
                               IntersectingAxes=[first, second],
                               OffsetDistances=[float(v) for v in offsets])

    def on_grid(location, reference=None):
        arguments = {"PlacementLocation": location, "PlacementRefDirection": reference}
        if schema != "IFC4":
            arguments["PlacementRelTo"] = grid_placement
        return f.create_entity("IfcGridPlacement", **arguments)

    a1 = on_grid(at(a, one, (0.5, -0.25, 1.0)))
    contained = [grid]
    for name, placement in [
        ("A1_OFFSET", a1),
        ("B2_DIRECTION", on_grid(at(b, two, (0.0, 0.0)), direction(f, (1.0, 1.0, 5.0)))),
        ("A1_TOWARDS_B2", on_grid(at(a, one, (0.0, 0.0)), at(b, two, (0.0, 0.0)))),
        ("B1_SAMESENSE", on_grid(at(b, one, (0.5, 0.0)))),
        ("R3_CURVED", on_grid(at(r, three, (0.5, 0.0)))),
        ("BRACKET", local(f, (1.0, 0.0, 2.0), a1)),
    ]:
        contained.append(proxy(f, label, name, placement, body_ctx))
    f.create_entity("IfcRelContainedInSpatialStructure", GlobalId=guid(label + "/contained"),
                    RelatingStructure=storey, RelatedElements=contained)
    return f


def alignment_file(label, precision, products):
    f = ifcopenshell.file(schema="IFC4X3_ADD2")
    ctx = context(f, precision)
    body_ctx = f.create_entity(
        "IfcGeometricRepresentationSubContext", ContextIdentifier="Body",
        ContextType="Model", ParentContext=ctx, TargetView="MODEL_VIEW")
    project = f.create_entity("IfcProject", GlobalId=guid(label + "/project"),
                              Name=label, UnitsInContext=units(f),
                              RepresentationContexts=[ctx])
    site = f.create_entity("IfcSite", GlobalId=guid(label + "/site"), Name="Site",
                           ObjectPlacement=local(f))
    plan, gradient = centreline(f)
    carrier = f.create_entity(
        "IfcBuildingElementProxy", GlobalId=guid(label + "/centreline"),
        Name="Alignment centreline carrier", ObjectPlacement=local(f),
        Representation=shape(f, [rep(f, ctx, "FootPrint", "Curve2D", [plan]),
                                 rep(f, ctx, "Axis", "Curve3D", [gradient])]))
    contained = [carrier]
    for name, arguments in products:
        block = f.create_entity("IfcBlock", Position=place3(f, (-0.5, -0.5, 0.0)),
                                XLength=1.0, YLength=1.0, ZLength=1.0)
        contained.append(f.create_entity(
            "IfcBuildingElementProxy", GlobalId=guid(label + "/" + name), Name=name,
            ObjectPlacement=along(f, gradient, *arguments(f)),
            Representation=shape(f, [rep(f, body_ctx, "Body", "CSG", [block])])))
    f.create_entity("IfcRelAggregates", GlobalId=guid(label + "/aggregates"),
                    RelatingObject=project, RelatedObjects=[site])
    f.create_entity("IfcRelContainedInSpatialStructure", GlobalId=guid(label + "/contained"),
                    RelatingStructure=site, RelatedElements=contained)
    return f


def uncached():
    def at_40(f):
        return (length(f, 40.0), 2.0)

    def axes(f):
        return (length(f, 40.0), 2.0, (0.0, 0.0, 1.0), (0.0, 1.0, 0.0))

    return alignment_file("uncached", 1e-6, [
        ("LENGTH", at_40), ("AXES", axes)])


def cache_check():
    def cached(position):
        def arguments(f):
            return (length(f, 40.0), 2.0, None, None, position)
        return arguments

    return alignment_file("cache-check", 1e-5, [
        ("MATCH", cached((40.0, 2.0, 10.8))),
        ("NEAR", cached((40.000004, 2.0, 10.8))),
        ("OUTSIDE", cached((40.00005, 2.0, 10.8))),
        ("STALE", cached((60.0, 2.0, 11.2))),
    ])


def spatial_root(f, label):
    """Project, context, Body sub-context and site of a profile fixture."""
    ctx = context(f, 1e-5)
    body_ctx = f.create_entity(
        "IfcGeometricRepresentationSubContext", ContextIdentifier="Body",
        ContextType="Model", ParentContext=ctx, TargetView="MODEL_VIEW")
    project = f.create_entity("IfcProject", GlobalId=guid(label + "/project"),
                              Name=label, UnitsInContext=units(f),
                              RepresentationContexts=[ctx])
    site = f.create_entity("IfcSite", GlobalId=guid(label + "/site"), Name="Site",
                           ObjectPlacement=local(f))
    f.create_entity("IfcRelAggregates", GlobalId=guid(label + "/aggregates"),
                    RelatingObject=project, RelatedObjects=[site])
    return body_ctx, site


def contain(f, label, site, products):
    f.create_entity("IfcRelContainedInSpatialStructure", GlobalId=guid(label + "/contained"),
                    RelatingStructure=site, RelatedElements=products)


# The D shape of `tests/composite_profile_compile.rs`: the square [-1, 1]^2
# whose right edge is a half circle of radius 1 about (1, 0). Area 4 + pi/2.
D_POINTS = [(-1.0, -1.0), (1.0, -1.0), (2.0, 0.0), (1.0, 1.0), (-1.0, 1.0)]


def point_list(f, points, tags=None):
    arguments = {"CoordList": [[float(x), float(y)] for x, y in points]}
    if tags is not None:
        arguments["TagList"] = tags
    return f.create_entity("IfcCartesianPointList2D", **arguments)


def indexed(f, points, segments=None, tags=None):
    """An `IfcIndexedPolyCurve`; `segments` are ("line" | "arc", 1-based indices)."""
    return f.create_entity(
        "IfcIndexedPolyCurve", Points=point_list(f, points, tags),
        Segments=None if segments is None else [
            f.create_entity("IfcLineIndex" if kind == "line" else "IfcArcIndex", indices)
            for kind, indices in segments],
        SelfIntersect=False)


def polyline2(f, points):
    return f.create_entity("IfcPolyline", Points=[point2(f, p) for p in points])


def composite(f, parts):
    """An `IfcCompositeCurve` of (parent, SameSense) segments."""
    return f.create_entity("IfcCompositeCurve", Segments=[
        f.create_entity("IfcCompositeCurveSegment", Transition="CONTINUOUS",
                        SameSense=same_sense, ParentCurve=parent)
        for parent, same_sense in parts], SelfIntersect=False)


def d_arc(f, start, end, sense):
    """The D's half circle about (1, 0), trimmed at Cartesian points."""
    circle = f.create_entity(
        "IfcCircle", Position=f.create_entity("IfcAxis2Placement2D",
                                              Location=point2(f, (1.0, 0.0))),
        Radius=1.0)
    return f.create_entity("IfcTrimmedCurve", BasisCurve=circle,
                           Trim1=[point2(f, start)], Trim2=[point2(f, end)],
                           SenseAgreement=sense, MasterRepresentation="CARTESIAN")


def extruded(f, profile, x):
    """`profile` extruded 1 m up from (x, 0, 0), so the volume is its area."""
    return f.create_entity(
        "IfcExtrudedAreaSolid", SweptArea=profile, Position=place3(f, (x, 0.0, 0.0)),
        ExtrudedDirection=direction(f, (0.0, 0.0, 1.0)), Depth=1.0)


def profile_boundaries(schema):
    """#335: arbitrary profiles bounded by `IfcIndexedPolyCurve`s, each next
    to the `IfcPolyline` or `IfcCompositeCurve` it must lower like."""
    label = "indexed-profile-" + schema
    f = ifcopenshell.file(schema=schema)
    body_ctx, site = spatial_root(f, label)
    tags = ["A", "B", "C", "D", "E"] if schema != "IFC4" else None

    def closed(curve):
        return f.create_entity("IfcArbitraryClosedProfileDef", ProfileType="AREA",
                               OuterCurve=curve)

    d_ccw = [("line", (1, 2)), ("arc", (2, 3, 4)), ("line", (4, 5, 1))]
    d_cw = [("line", (1, 5, 4)), ("arc", (4, 3, 2)), ("line", (2, 1))]
    square = [(0.0, 0.0), (2.0, 0.0), (2.0, 1.0), (0.0, 1.0), (0.0, 0.0)]
    ring = [(-2.0, -2.0), (2.0, -2.0), (2.0, 2.0), (-2.0, 2.0)]
    circle = [(1.0, 0.0), (0.0, 1.0), (-1.0, 0.0), (0.0, -1.0)]
    profiles = [
        ("INDEXED_D", closed(indexed(f, D_POINTS, d_ccw, tags))),
        ("COMPOSITE_D", closed(composite(f, [
            (polyline2(f, D_POINTS[0:2]), True),
            (d_arc(f, (1.0, -1.0), (1.0, 1.0), True), True),
            (polyline2(f, D_POINTS[3:5] + D_POINTS[0:1]), True)]))),
        ("INDEXED_D_CLOCKWISE", closed(indexed(f, D_POINTS, d_cw, tags))),
        ("COMPOSITE_D_CLOCKWISE", closed(composite(f, [
            (polyline2(f, [D_POINTS[0], D_POINTS[4], D_POINTS[3]]), True),
            (d_arc(f, (1.0, 1.0), (1.0, -1.0), False), True),
            (polyline2(f, [D_POINTS[1], D_POINTS[0]]), True)]))),
        ("INDEXED_POLYLINE", closed(indexed(f, square))),
        ("POLYLINE", closed(polyline2(f, square))),
        ("INDEXED_WITH_VOIDS", f.create_entity(
            "IfcArbitraryProfileDefWithVoids", ProfileType="AREA",
            OuterCurve=indexed(f, ring, [("line", (1, 2, 3, 4, 1))]),
            InnerCurves=[indexed(f, circle, [("arc", (1, 2, 3)), ("arc", (3, 4, 1))])])),
    ]
    products = []
    for index, (name, profile) in enumerate(profiles):
        profile.ProfileName = name
        products.append(f.create_entity(
            "IfcBuildingElementProxy", GlobalId=guid(label + "/" + name), Name=name,
            ObjectPlacement=local(f),
            Representation=shape(f, [rep(f, body_ctx, "Body", "SweptSolid",
                                         [extruded(f, profile, 10.0 * index)])])))
    contain(f, label, site, products)
    return f


def curve_bounded_plane_composite():
    """#336: `IfcCurveBoundedPlane`s bounded by composites of polylines."""
    label = "curve-bounded-composite"
    f = ifcopenshell.file(schema="IFC4")
    body_ctx, site = spatial_root(f, label)

    def plane(z):
        return f.create_entity("IfcPlane", Position=place3(f, (0.0, 0.0, z)))

    # 4 x 3 with a 1 x 1 hole: area 11. The middle outer segment is authored
    # backwards and read with SameSense FALSE.
    outer = composite(f, [
        (polyline2(f, [(0.0, 0.0), (4.0, 0.0), (4.0, 1.0)]), True),
        (polyline2(f, [(4.0, 3.0), (4.0, 1.0)]), False),
        (polyline2(f, [(4.0, 3.0), (0.0, 3.0), (0.0, 0.0)]), True)])
    hole = composite(f, [
        (polyline2(f, [(1.0, 1.0), (2.0, 1.0), (2.0, 2.0)]), True),
        (polyline2(f, [(2.0, 2.0), (1.0, 2.0), (1.0, 1.0)]), True)])
    # One segment, SameSense FALSE, wrapping a clockwise closed polyline:
    # read backwards it is the anticlockwise 4 x 3 rectangle, area 12.
    reversed_ring = composite(f, [(polyline2(
        f, [(0.0, 0.0), (0.0, 3.0), (4.0, 3.0), (4.0, 0.0), (0.0, 0.0)]), False)])
    surfaces = [
        ("SEGMENTS_WITH_HOLE", f.create_entity(
            "IfcCurveBoundedPlane", BasisSurface=plane(0.0), OuterBoundary=outer,
            InnerBoundaries=[hole])),
        ("ONE_REVERSED_SEGMENT", f.create_entity(
            "IfcCurveBoundedPlane", BasisSurface=plane(5.0),
            OuterBoundary=reversed_ring, InnerBoundaries=[])),
    ]
    products = [
        f.create_entity(
            "IfcBuildingElementProxy", GlobalId=guid(label + "/" + name), Name=name,
            ObjectPlacement=local(f),
            Representation=shape(f, [rep(f, body_ctx, "Body", "Surface3D", [surface])]))
        for name, surface in surfaces]
    contain(f, label, site, products)
    return f

# #393: the boundary every clipped wall is cut by, anticlockwise: a 2 x 1.5
# rectangle under a gable reaching (2, 2). The wall is [0, 4] x [0, 1], so
# the footprint it removes is x in [1, 3], y in [0, 0.5] (area 1) plus the
# gable's part below y = 1 (area 5/6): 11/6.
HALFSPACE_BOUNDARY = [(1.0, -1.0), (3.0, -1.0), (3.0, 0.5), (2.0, 2.0), (1.0, 0.5)]


def halfspace_boundaries():
    """#393: walls clipped by polygonal bounded half-spaces whose boundary is
    an `IfcCompositeCurve` or `IfcIndexedPolyCurve`, each beside the wall
    whose boundary is the equivalent `IfcPolyline`."""
    label = "halfspace-boundaries"
    f = ifcopenshell.file(schema="IFC4X3_ADD2")
    body_ctx, site = spatial_root(f, label)
    v = HALFSPACE_BOUNDARY
    closed = v + v[:1]

    def trimmed_line(origin, heading, length):
        return f.create_entity(
            "IfcTrimmedCurve", BasisCurve=line_through(f, origin, heading),
            Trim1=[f.create_entity("IfcParameterValue", 0.0)],
            Trim2=[f.create_entity("IfcParameterValue", float(length))],
            SenseAgreement=True, MasterRepresentation="PARAMETER")

    # Closed as conforming exporters close a polyline: by repeating the
    # first point's reference.
    twin = [point2(f, p) for p in v]
    boundaries = [
        ("POLYLINE", f.create_entity("IfcPolyline", Points=twin + twin[:1])),
        # The gable's two edges authored backwards, read with SameSense FALSE.
        ("COMPOSITE", composite(f, [
            (polyline2(f, v[0:3]), True),
            (polyline2(f, [v[4], v[3], v[2]]), False),
            (polyline2(f, [v[4], v[0]]), True)])),
        # The closing edge as an IfcTrimmedCurve on an IfcLine.
        ("COMPOSITE_LINE", composite(f, [
            (polyline2(f, v), True),
            (trimmed_line(v[4], (0.0, -1.0), 1.5), True)])),
        ("INDEXED", indexed(f, closed)),
        ("INDEXED_SEGMENTS", indexed(f, v, [
            ("line", (1, 2, 3)), ("line", (3, 4)), ("line", (4, 5, 1))])),
    ]

    def wall(index, name, boundary):
        # A 4 x 1 x 3 wall, cut above z = 2 inside the boundary's prism.
        profile = f.create_entity(
            "IfcRectangleProfileDef", ProfileType="AREA", XDim=4.0, YDim=1.0,
            Position=f.create_entity("IfcAxis2Placement2D",
                                     Location=point2(f, (2.0, 0.5))))
        body = f.create_entity(
            "IfcExtrudedAreaSolid", SweptArea=profile, Position=place3(f),
            ExtrudedDirection=direction(f, (0.0, 0.0, 1.0)), Depth=3.0)
        clip = f.create_entity(
            "IfcPolygonalBoundedHalfSpace",
            BaseSurface=f.create_entity("IfcPlane", Position=place3(f, (0.0, 0.0, 2.0))),
            AgreementFlag=False, Position=place3(f), PolygonalBoundary=boundary)
        result = f.create_entity("IfcBooleanClippingResult", Operator="DIFFERENCE",
                                 FirstOperand=body, SecondOperand=clip)
        return f.create_entity(
            "IfcWall", GlobalId=guid(label + "/" + name), Name=name,
            ObjectPlacement=local(f, (10.0 * index, 0.0, 0.0)), PredefinedType="NOTDEFINED",
            Representation=shape(f, [rep(f, body_ctx, "Body", "Clipping", [result])]))

    walls = [wall(index, name, boundary) for index, (name, boundary) in enumerate(boundaries)]
    # #396: the gable edge as an IfcArcIndex through its midpoint, collinear,
    # so "treated as a polyline segment". Created after the five walls so
    # their ids stay as #393 committed them.
    collinear = indexed(f, v + [(2.5, 1.25)], [
        ("line", (1, 2, 3)), ("arc", (3, 6, 4)), ("line", (4, 5, 1))])
    walls.append(wall(len(walls), "INDEXED_COLLINEAR_ARC", collinear))

    # #398: boundaries with genuine circular arcs, lowered as exact profile
    # contours. Created last so the ids above stay as #393 and #396
    # committed them.
    #
    # COMPOSITE_ARCS: six segments, two of them arcs over one IfcCircle of
    # radius 1.2 about (1, 0.5), as the downstream wall's boundary
    # (axioval/engine#307): the rectangle [1, 3.5] x [-0.22, 2.5] less that
    # disk, run anticlockwise, the bite clockwise and split at (2.2, 0.5).
    # The bottom edge lies 0.72 below the centre, so the arc meets it at
    # (1.96, -0.22) exactly.
    circle = f.create_entity(
        "IfcCircle", Position=f.create_entity("IfcAxis2Placement2D",
                                              Location=point2(f, (1.0, 0.5))),
        Radius=1.2)
    # Authored 0 -> pi/2 by parameter and walked backwards by SameSense FALSE.
    upper = f.create_entity(
        "IfcTrimmedCurve", BasisCurve=circle,
        Trim1=[f.create_entity("IfcParameterValue", 0.0)],
        Trim2=[f.create_entity("IfcParameterValue", math.pi / 2.0)],
        SenseAgreement=True, MasterRepresentation="PARAMETER")
    # Authored clockwise by Cartesian points.
    lower = f.create_entity(
        "IfcTrimmedCurve", BasisCurve=circle,
        Trim1=[point2(f, (2.2, 0.5))], Trim2=[point2(f, (1.96, -0.22))],
        SenseAgreement=False, MasterRepresentation="CARTESIAN")
    arcs = composite(f, [
        (polyline2(f, [(1.96, -0.22), (3.5, -0.22)]), True),
        (polyline2(f, [(3.5, -0.22), (3.5, 2.5)]), True),
        (polyline2(f, [(3.5, 2.5), (1.0, 2.5)]), True),
        (polyline2(f, [(1.0, 2.5), (1.0, 1.7)]), True),
        (upper, False),
        (lower, True)])
    walls.append(wall(len(walls), "COMPOSITE_ARCS", arcs))
    # INDEXED_ARC: the 2 x 1.5 rectangle under a half circle of radius 1
    # about (2, 0.5), the IfcArcIndex (3, 4, 5) through its top (2, 1.5).
    semicircle = indexed(f, [(1.0, -1.0), (3.0, -1.0), (3.0, 0.5), (2.0, 1.5), (1.0, 0.5)], [
        ("line", (1, 2, 3)), ("arc", (3, 4, 5)), ("line", (5, 1))])
    walls.append(wall(len(walls), "INDEXED_ARC", semicircle))
    contain(f, label, site, walls)
    return f


def indexed_curve_arcs():
    """#396: `IfcIndexedPolyCurve` representation curves whose `IfcArcIndex`
    is collinear, nearly collinear or a genuine arc."""
    label = "indexed-curve-arcs"
    f = ifcopenshell.file(schema="IFC4")
    _, site = spatial_root(f, label)
    model_ctx = f.by_type("IfcGeometricRepresentationContext", include_subtypes=False)[0]
    axis_ctx = f.create_entity(
        "IfcGeometricRepresentationSubContext", ContextIdentifier="Axis",
        ContextType="Model", ParentContext=model_ctx, TargetView="GRAPH_VIEW")

    def curve(points, segments):
        return f.create_entity(
            "IfcIndexedPolyCurve",
            Points=f.create_entity("IfcCartesianPointList3D", CoordList=[
                [float(c) for c in p] for p in points]),
            Segments=[
                f.create_entity("IfcLineIndex" if kind == "line" else "IfcArcIndex", indices)
                for kind, indices in segments],
            SelfIntersect=False)

    curves = [
        # The middle point between the others: one straight edge (2,0)-(4,0).
        ("COLLINEAR_ARC", curve(
            [(0, 0, 0), (2, 0, 0), (3, 0, 0), (4, 0, 0), (4, 2, 0)],
            [("line", (1, 2)), ("arc", (2, 3, 4)), ("line", (4, 5))])),
        # The middle point beyond the end: out to (4,0,1) and back to (2,0,1).
        ("OUT_AND_BACK_ARC", curve(
            [(0, 0, 1), (4, 0, 1), (2, 0, 1), (2, 2, 1)],
            [("arc", (1, 2, 3)), ("line", (3, 4))])),
        # The middle point 4e-6 m off the chord, inside the 1e-5 m Precision.
        ("NEAR_COLLINEAR_ARC", curve(
            [(0, 0, 2), (1, 0.000004, 2), (2, 0, 2)],
            [("arc", (1, 2, 3))])),
        # A genuine half circle of radius 1 about (1, 0, 3).
        ("ARC", curve(
            [(0, 0, 3), (1, 1, 3), (2, 0, 3)],
            [("arc", (1, 2, 3))])),
    ]
    products = [
        f.create_entity(
            "IfcBuildingElementProxy", GlobalId=guid(label + "/" + name), Name=name,
            ObjectPlacement=local(f),
            Representation=shape(f, [rep(f, axis_ctx, "Axis", "Curve3D", [item])]))
        for name, item in curves]
    contain(f, label, site, products)
    return f


# #346: the grade break and the corner the station-seam fixture straddles.
SEAM_GRADES = ((40.0, 0.02), (60.0, -0.01))
SEAM_START_HEIGHT = 10.0


def station_seams():
    """#346: stations, sections and offsets on and across tangent
    discontinuities of a gradient curve and of a polyline."""
    label = "station-seams"
    f = ifcopenshell.file(schema="IFC4X3_ADD2")
    body_ctx, site = spatial_root(f, label)
    ctx = f.by_type("IfcGeometricRepresentationContext", include_subtypes=False)[0]

    # The gradient curve: a straight plan, two grades with no vertical curve.
    line = line2(f)
    plan_length = sum(run for run, _ in SEAM_GRADES)
    plan = f.create_entity("IfcCompositeCurve", Segments=[
        segment(f, (0.0, 0.0), (1.0, 0.0), plan_length, line),
        segment(f, (plan_length, 0.0), (1.0, 0.0), 0.0, line, "DISCONTINUOUS"),
    ], SelfIntersect=False)
    profile = []
    at, height = 0.0, SEAM_START_HEIGHT
    for run, grade in SEAM_GRADES:
        norm = math.hypot(1.0, grade)
        profile.append(segment(f, (at, height), (1.0 / norm, grade / norm), run * norm, line))
        at, height = at + run, height + grade * run
    last = SEAM_GRADES[-1][1]
    norm = math.hypot(1.0, last)
    profile.append(segment(f, (at, height), (1.0 / norm, last / norm), 0.0, line,
                           "DISCONTINUOUS"))
    gradient = f.create_entity("IfcGradientCurve", Segments=profile, SelfIntersect=False,
                               BaseCurve=plan)

    corner = f.create_entity("IfcPolyline", Points=[
        point(f, (0.0, 0.0, 0.0)), point(f, (10.0, 0.0, 0.0)), point(f, (10.0, 10.0, 0.0))])

    def along(basis, distance, lateral=None, vertical=None, longitudinal=None):
        return f.create_entity(
            "IfcPointByDistanceExpression", DistanceAlong=length(f, distance),
            OffsetLateral=lateral, OffsetVertical=vertical,
            OffsetLongitudinal=longitudinal, BasisCurve=basis)

    def position(basis, distance, **offsets):
        return f.create_entity("IfcAxis2PlacementLinear",
                               Location=along(basis, distance, **offsets))

    def product(name, representations):
        return f.create_entity(
            "IfcBuildingElementProxy", GlobalId=guid(label + "/" + name), Name=name,
            ObjectPlacement=local(f), Representation=shape(f, representations))

    def points(name, item):
        return product(name, [rep(f, ctx, "Reference", "Point", [item])])

    rectangle = f.create_entity(
        "IfcRectangleProfileDef", ProfileType="AREA", ProfileName="deck",
        Position=f.create_entity("IfcAxis2Placement2D", Location=point2(f, (0.0, 0.0))),
        XDim=2.0, YDim=1.0)
    deck = f.create_entity(
        "IfcSectionedSolidHorizontal", Directrix=corner,
        CrossSections=[rectangle, rectangle],
        CrossSectionPositions=[position(corner, 5.0), position(corner, 15.0)])

    kerb = f.create_entity(
        "IfcOffsetCurveByDistances", BasisCurve=corner,
        OffsetValues=[along(corner, 2.0, lateral=1.0), along(corner, 18.0, lateral=1.0)],
        Tag="kerb")
    tube = f.create_entity("IfcSweptDiskSolid", Directrix=kerb, Radius=0.1,
                           StartParam=0.0, EndParam=20.0)

    def flat():
        return f.create_entity(
            "IfcOpenCrossProfileDef", ProfileType="CURVE", HorizontalWidths=True,
            Widths=[3.5, 3.5], Slopes=[0.0, 0.0], Tags=["l", "c", "r"],
            OffsetPoint=point2(f, (-3.5, 0.0)))
    carriageway = f.create_entity(
        "IfcSectionedSurface", Directrix=gradient,
        CrossSectionPositions=[position(gradient, 30.0), position(gradient, 50.0)],
        CrossSections=[flat(), flat()])

    products = [
        product("ALIGNMENT", [rep(f, ctx, "FootPrint", "Curve2D", [plan]),
                              rep(f, ctx, "Axis", "Curve3D", [gradient])]),
        points("GRADE_BREAK_AT", along(gradient, 40.0, vertical=2.0)),
        points("GRADE_BREAK_NEAR", along(gradient, 40.000004, vertical=2.0)),
        points("CORNER_AT", along(corner, 10.0, lateral=1.0, longitudinal=0.5)),
        points("CORNER_NEAR", along(corner, 9.999996, lateral=1.0, longitudinal=0.5)),
        product("DECK", [rep(f, body_ctx, "Body", "AdvancedSweptSolid", [deck])]),
        product("KERB", [rep(f, ctx, "Axis", "Curve3D", [kerb]),
                         rep(f, body_ctx, "Body", "AdvancedSweptSolid", [tube])]),
        product("CARRIAGEWAY", [rep(f, body_ctx, "Body", "SectionedSurface", [carriageway])]),
    ]
    contain(f, label, site, products)
    return f


# #388: the site placements the flush-opening wall is put under. A survey
# origin of (600 000, 5 600 000, 200) and a turn of about 2.3 degrees to
# grid north are what a georeferenced export writes.
GEO_ORIGIN = (600000.0, 5600000.0, 200.0)
GEO_TURN = (0.999215271103513, 0.0396086100934191, 0.0)
GEO_SITES = [
    ("ORIGIN", (0.0, 0.0, 0.0), None),
    ("ORIGIN_TURNED", (0.0, 0.0, 0.0), GEO_TURN),
    ("GEOREF", GEO_ORIGIN, None),
    ("GEOREF_TURNED", GEO_ORIGIN, GEO_TURN),
]
# The wall in its site: at (13.475, -15.95, 4.3), turned 90 degrees, so
# its x runs along the site's y and its y along the site's -x.
GEO_WALL_AT = (13.475, -15.95, 4.3)
# The openings in the wall's frame, flush with both faces (y = +-0.125).
GEO_OPENINGS_AT = [(3.325, 0.125, 0.2), (1.35, 0.125, 0.2)]
# The same openings in the SITE's frame: wall origin + (-y, x, z).
GEO_OPENINGS_IN_SITE = [(13.35, -12.625, 4.5), (13.35, -14.6, 4.5)]


def flush_openings():
    """#388: a wall with two flush openings under four site placements."""
    label = "flush-openings"
    f = ifcopenshell.file(schema="IFC4")
    ctx = context(f, 1e-5)
    body_ctx = f.create_entity(
        "IfcGeometricRepresentationSubContext", ContextIdentifier="Body",
        ContextType="Model", ParentContext=ctx, TargetView="MODEL_VIEW")
    project = f.create_entity("IfcProject", GlobalId=guid(label + "/project"),
                              Name=label, UnitsInContext=units(f),
                              RepresentationContexts=[ctx])

    def wall_body(parts):
        # `parts` as (length, centre) along the wall's x.
        solids = []
        for length, centre in parts:
            profile = f.create_entity(
                "IfcRectangleProfileDef", ProfileType="AREA", XDim=length, YDim=0.25,
                Position=f.create_entity(
                    "IfcAxis2Placement2D", Location=f.create_entity(
                        "IfcCartesianPoint", Coordinates=[centre, 0.0]),
                    RefDirection=direction(f, (-1.0, 0.0))))
            solids.append(f.create_entity(
                "IfcExtrudedAreaSolid", SweptArea=profile, Position=place3(f),
                ExtrudedDirection=direction(f, (0.0, 0.0, 1.0)), Depth=3.67))
        return shape(f, [rep(f, body_ctx, "Body", "SweptSolid", solids)])

    def opening_body():
        profile = f.create_entity(
            "IfcRectangleProfileDef", ProfileType="AREA", XDim=1.01, YDim=2.26,
            Position=f.create_entity(
                "IfcAxis2Placement2D", Location=f.create_entity(
                    "IfcCartesianPoint", Coordinates=[1.13, 0.505]),
                RefDirection=direction(f, (0.0, 1.0))))
        solid = f.create_entity(
            "IfcExtrudedAreaSolid", SweptArea=profile,
            Position=place3(f, axis=(0.0, -1.0, 0.0), ref=(0.0, 0.0, 1.0)),
            ExtrudedDirection=direction(f, (0.0, 0.0, 1.0)), Depth=0.25)
        return shape(f, [rep(f, body_ctx, "Body", "SweptSolid", [solid])])

    def wall(name, site_placement, opening_placements, parts=((3.65, 1.825),)):
        placement = f.create_entity(
            "IfcLocalPlacement", PlacementRelTo=site_placement,
            RelativePlacement=place3(f, GEO_WALL_AT, axis=(0.0, 0.0, 1.0),
                                     ref=(0.0, 1.0, 0.0)))
        element = f.create_entity(
            "IfcWall", GlobalId=guid(label + "/" + name), Name=name,
            ObjectPlacement=placement, PredefinedType="NOTDEFINED",
            Representation=wall_body(parts))
        for index, opening_placement in enumerate(opening_placements(placement)):
            opening = f.create_entity(
                "IfcOpeningElement", GlobalId=guid(f"{label}/{name}/opening-{index}"),
                Name=f"{name}/{index}", ObjectPlacement=opening_placement,
                PredefinedType="OPENING", Representation=opening_body())
            f.create_entity(
                "IfcRelVoidsElement", GlobalId=guid(f"{label}/{name}/voids-{index}"),
                RelatingBuildingElement=element, RelatedOpeningElement=opening)
        return element

    def in_wall(wall_placement):
        return [local(f, at, wall_placement) for at in GEO_OPENINGS_AT]

    def in_frame(parent):
        # The wall's frame turned into its site's: the opening's x runs
        # along the site's y, as the wall's does.
        return lambda _wall: [f.create_entity(
            "IfcLocalPlacement", PlacementRelTo=parent,
            RelativePlacement=place3(f, at, axis=(0.0, 0.0, 1.0), ref=(0.0, 1.0, 0.0)))
            for at in GEO_OPENINGS_IN_SITE]

    sites = []
    for name, origin, turn in GEO_SITES:
        site_placement = f.create_entity(
            "IfcLocalPlacement", PlacementRelTo=None,
            RelativePlacement=place3(f, origin, axis=(0.0, 0.0, 1.0) if turn else None,
                                     ref=turn))
        site = f.create_entity("IfcSite", GlobalId=guid(label + "/site/" + name),
                               Name=name, ObjectPlacement=site_placement)
        sites.append(site)
        walls = [wall(name, site_placement, in_wall)]
        if name == "GEOREF_TURNED":
            # Openings placed relative to the SITE, not the wall: the
            # placement the two share is the site's.
            walls.append(wall("SIBLING", site_placement, in_frame(site_placement)))
            # The same wall as two solids meeting at x = 1.825, between the
            # openings: each part is cut in the wall's frame.
            walls.append(wall("TWO_PART", site_placement, in_wall,
                              parts=((1.825, 0.9125), (1.825, 2.7375))))
        if name == "ORIGIN":
            # Openings placed absolutely: no placement shared with the wall.
            # The site is the identity, so site coordinates are world ones.
            walls.append(wall("ABSOLUTE", site_placement, in_frame(None)))
        contain(f, f"{label}/{name}", site, walls)
    f.create_entity("IfcRelAggregates", GlobalId=guid(label + "/aggregates"),
                    RelatingObject=project, RelatedObjects=sites)
    return f


FIXTURES = {
    "reference_view_openings.ifc": reference_view,
    "linear_placement_uncached.ifc": uncached,
    "linear_placement_cache_check.ifc": cache_check,
    "linear_placement_alignment_frame.ifc": alignment_frame,
    "grid_placement.ifc": lambda: grid_file("IFC4"),
    "grid_placement_ifc4x3.ifc": lambda: grid_file("IFC4X3_ADD2"),
    "indexed_profile_boundaries.ifc": lambda: profile_boundaries("IFC4"),
    "indexed_profile_boundaries_ifc4x3.ifc": lambda: profile_boundaries("IFC4X3_ADD2"),
    "curve_bounded_plane_composite.ifc": curve_bounded_plane_composite,
    "flush_openings_site_placements.ifc": flush_openings,
    "halfspace_boundaries_ifc4x3.ifc": halfspace_boundaries,
    "indexed_curve_arcs.ifc": indexed_curve_arcs,
    "station_seams_ifc4x3.ifc": station_seams,
}


def main():
    if len(sys.argv) != 2:
        print("usage: gen_lowering_fixtures.py <outdir>", file=sys.stderr)
        return 2
    out = pathlib.Path(sys.argv[1])
    out.mkdir(parents=True, exist_ok=True)
    for name, build in FIXTURES.items():
        model = build()
        model.header.file_name.name = name
        model.header.file_name.time_stamp = "1970-01-01T00:00:00"
        target = out / name
        model.write(str(target))
        print(f"{target.stat().st_size:>7} {name}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
