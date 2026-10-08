#!/usr/bin/env python3
"""Generate the product-lowering fixtures for #351, #353, #354, #357, #362
and #363.

Six small files, each one edge case of `crates/ifc-geometry`'s product
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


FIXTURES = {
    "reference_view_openings.ifc": reference_view,
    "linear_placement_uncached.ifc": uncached,
    "linear_placement_cache_check.ifc": cache_check,
    "linear_placement_alignment_frame.ifc": alignment_frame,
    "grid_placement.ifc": lambda: grid_file("IFC4"),
    "grid_placement_ifc4x3.ifc": lambda: grid_file("IFC4X3_ADD2"),
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
