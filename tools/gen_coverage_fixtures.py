#!/usr/bin/env python3
"""Generate the #47 product-meshing coverage fixture with ifcopenshell.

Issue #47 measured six real building models and sorted every product that
failed to mesh into six failure kinds. The real models cannot be committed
(no redistribution licence), so this file restates each failure kind as ONE
small product. `ifc-geometry/tests/meshing_coverage.rs` compiles each through
`compile_product_mesh` (and `compile_product_mesh_reported`) and pins the
answer it gets today, so a regression in any kind fails CI.

One product per kind, each named by its kind so the test finds it by name:

| Name                         | Kind (#47 row)                          |
| ---------------------------- | --------------------------------------- |
| composite-curve-profile      | IfcCompositeCurve profile boundary, #43 |
| wall-with-opening            | IfcRelVoidsElement never subtracted, #44 |
| bounded-half-space-clip      | IfcPolygonalBoundedHalfSpace, #45       |
| collapsed-poly-loop          | degenerate IfcPolyLoop face, #46        |
| polygonal-face-set-quads     | faces with > 3 corners, kernel#160      |
| shell-based-surface-model    | open-shell surface model, kernel#161    |

Every value is chosen so the expected volume is exact and distinct:

- composite-curve-profile: a "D" of a 1 x 2 m rectangle plus a half disc of
  radius 1 m, extruded 1 m: 2 + pi/2 m3. The arc stays exact in lowering;
  the mesh is within the kernel chord budget of it.
- wall-with-opening: a 4 x 0.3 x 3 m wall (3.6 m3) voided by a 1 x 0.3 x 2 m
  door that is FLUSH with both wall faces: net 3.0 m3. The opening body is
  extruded DOWNWARD from its lintel, as Solibri and Revit author openings,
  so the axiolid/kernel#166 path is covered too.
- bounded-half-space-clip: a 4 x 6 x 2 m box clipped above z = 1.5 m by a
  boundary whose Position is translated to (0, 2.75) within the clip plane;
  footprint x in [1, 2], y in [2.75, 3] -> removes 0.125 m3, leaves 47.875 m3.
  The translation is the axiolid/kernel#164 path.
- collapsed-poly-loop: a closed 2 x 3 x 4 m faceted brep (24 m3) with one
  extra face whose loop is (A, A, B, B), the exact shape of the 16 refused
  OfficeBuilding windows.
- polygonal-face-set-quads: a closed 2 m cube (8 m3) as an
  IfcPolygonalFaceSet of six QUAD faces.
- shell-based-surface-model: two triangles in an IfcOpenShell. It asserts no
  volume and must never acquire one.

Authored in millimetres with degree angles, like the surface fixtures, so a
unit factor applied to the wrong quantity cannot pass. Entity ids, GUIDs and
the header timestamp are fixed so regenerating produces the same bytes.

Run:  python3 tools/gen_coverage_fixtures.py test/fixtures/synthetic-coverage
"""

import pathlib
import sys
import uuid

import ifcopenshell
import ifcopenshell.guid

SCHEMA = "IFC4"
MM = 1000.0
NAME = "meshing_coverage.ifc"


def guid(label):
    """A GUID derived from a label, so regeneration is byte-stable."""
    return ifcopenshell.guid.compress(uuid.uuid5(uuid.NAMESPACE_URL, "openbimrs/ifc#47/" + label).hex)


def pt(f, xyz):
    return f.create_entity("IfcCartesianPoint", Coordinates=[float(v) * MM for v in xyz])


def pt2(f, xy):
    return f.create_entity("IfcCartesianPoint", Coordinates=[float(v) * MM for v in xy])


def dr(f, xyz):
    return f.create_entity("IfcDirection", DirectionRatios=[float(v) for v in xyz])


def placement(f, origin=(0.0, 0.0, 0.0), axis=None, ref=None):
    kw = {"Location": pt(f, origin)}
    if axis is not None:
        kw["Axis"] = dr(f, axis)
    if ref is not None:
        kw["RefDirection"] = dr(f, ref)
    return f.create_entity("IfcAxis2Placement3D", **kw)


def placement2(f, origin=(0.0, 0.0)):
    return f.create_entity("IfcAxis2Placement2D", Location=pt2(f, origin))


def local(f, relative_to=None, origin=(0.0, 0.0, 0.0)):
    return f.create_entity("IfcLocalPlacement", PlacementRelTo=relative_to,
                           RelativePlacement=placement(f, origin))


def units_and_context(f):
    mm = f.create_entity("IfcSIUnit", UnitType="LENGTHUNIT", Prefix="MILLI", Name="METRE")
    rad = f.create_entity("IfcSIUnit", UnitType="PLANEANGLEUNIT", Name="RADIAN")
    deg_ratio = f.create_entity(
        "IfcMeasureWithUnit",
        ValueComponent=f.create_entity("IfcPlaneAngleMeasure", 0.017453292519943295),
        UnitComponent=rad)
    deg = f.create_entity(
        "IfcConversionBasedUnit",
        Dimensions=f.create_entity("IfcDimensionalExponents", 0, 0, 0, 0, 0, 0, 0),
        UnitType="PLANEANGLEUNIT", Name="DEGREE", ConversionFactor=deg_ratio)
    assignment = f.create_entity("IfcUnitAssignment", Units=[mm, deg])
    ctx = f.create_entity("IfcGeometricRepresentationContext", ContextType="Model",
                          CoordinateSpaceDimension=3, Precision=1e-5,
                          WorldCoordinateSystem=placement(f),
                          TrueNorth=dr(f, (0.0, 1.0, 0.0)))
    body = f.create_entity("IfcGeometricRepresentationSubContext",
                           ContextIdentifier="Body", ContextType="Model",
                           ParentContext=ctx, TargetView="MODEL_VIEW")
    return assignment, ctx, body


def shape(f, body, items, rep_type):
    rep = f.create_entity("IfcShapeRepresentation", ContextOfItems=body,
                          RepresentationIdentifier="Body",
                          RepresentationType=rep_type, Items=items)
    return f.create_entity("IfcProductDefinitionShape", Representations=[rep])


def product(f, entity, name, body, items, rep_type, storey_place):
    return f.create_entity(entity, GlobalId=guid(name), Name=name,
                           ObjectPlacement=local(f, storey_place),
                           Representation=shape(f, body, items, rep_type))


def extrusion(f, profile, depth, position=None, direction=(0.0, 0.0, 1.0)):
    return f.create_entity("IfcExtrudedAreaSolid", SweptArea=profile,
                           Position=position or placement(f),
                           ExtrudedDirection=dr(f, direction),
                           Depth=float(depth) * MM)


def rectangle(f, x, y, centre=(0.0, 0.0)):
    return f.create_entity("IfcRectangleProfileDef", ProfileType="AREA",
                           Position=placement2(f, centre),
                           XDim=float(x) * MM, YDim=float(y) * MM)


# -- one builder per #47 failure kind --------------------------------------


def composite_curve_profile(f):
    """#43: a D of polyline + trimmed-arc segments, extruded 1 m.

    The closing polyline is authored BACKWARDS (points d-to-a order reversed,
    `SameSense=.F.`), the way many exporters write composite profiles. A
    lowerer that ignores `SameSense` then leaves a gap at both ends of that
    segment and the profile is refused, so the flag is observable.
    """
    a, b = pt2(f, (-1.0, -1.0)), pt2(f, (0.0, -1.0))
    c, d = pt2(f, (0.0, 1.0)), pt2(f, (-1.0, 1.0))
    circle = f.create_entity("IfcCircle", Position=placement2(f), Radius=1.0 * MM)
    arc = f.create_entity(
        "IfcTrimmedCurve", BasisCurve=circle,
        Trim1=[f.create_entity("IfcParameterValue", 270.0)],
        Trim2=[f.create_entity("IfcParameterValue", 90.0)],
        SenseAgreement=True, MasterRepresentation="PARAMETER")
    segments = [
        f.create_entity("IfcCompositeCurveSegment", Transition="CONTINUOUS", SameSense=True,
                        ParentCurve=f.create_entity("IfcPolyline", Points=[a, b])),
        f.create_entity("IfcCompositeCurveSegment", Transition="CONTINUOUS", SameSense=True,
                        ParentCurve=arc),
        f.create_entity("IfcCompositeCurveSegment", Transition="CONTINUOUS", SameSense=False,
                        ParentCurve=f.create_entity("IfcPolyline", Points=[a, d, c])),
    ]
    curve = f.create_entity("IfcCompositeCurve", Segments=segments, SelfIntersect=False)
    profile = f.create_entity("IfcArbitraryClosedProfileDef", ProfileType="AREA",
                              OuterCurve=curve)
    return [extrusion(f, profile, 1.0)], "SweptSolid"


def bounded_half_space_clip(f):
    """#45: a box clipped by a polygonal bounded half-space, Position translated."""
    box = extrusion(f, rectangle(f, 4.0, 6.0), 2.0)
    plane = f.create_entity("IfcPlane", Position=placement(f, (0.0, 0.0, 1.5)))
    boundary = f.create_entity("IfcPolyline", Points=[
        pt2(f, (1.0, 0.0)), pt2(f, (4.0, 0.0)), pt2(f, (4.0, 0.5)),
        pt2(f, (1.0, 0.5)), pt2(f, (1.0, 0.0))])
    half_space = f.create_entity(
        "IfcPolygonalBoundedHalfSpace", BaseSurface=plane, AgreementFlag=False,
        Position=placement(f, (0.0, 2.75, 0.0), axis=(0.0, 0.0, 1.0), ref=(1.0, 0.0, 0.0)),
        PolygonalBoundary=boundary)
    clip = f.create_entity("IfcBooleanClippingResult", Operator="DIFFERENCE",
                           FirstOperand=box, SecondOperand=half_space)
    return [clip], "Clipping"


# The six outward faces of an axis-aligned box, as corner indices into
# BOX_CORNERS. Each is counter-clockwise seen from outside, so the right-hand
# normal points out and the solid has positive volume.
BOX_FACES = [
    (0, 3, 2, 1),  # z = 0, normal -Z
    (4, 5, 6, 7),  # z = top, normal +Z
    (0, 1, 5, 4),  # y = 0, normal -Y
    (3, 7, 6, 2),  # y = top, normal +Y
    (0, 4, 7, 3),  # x = 0, normal -X
    (1, 2, 6, 5),  # x = top, normal +X
]


def box_corners(x, y, z):
    return [(0, 0, 0), (x, 0, 0), (x, y, 0), (0, y, 0),
            (0, 0, z), (x, 0, z), (x, y, z), (0, y, z)]


def collapsed_poly_loop(f):
    """#46: a closed 2 x 3 x 4 m faceted brep plus one (A, A, B, B) sliver face."""
    points = [pt(f, c) for c in box_corners(2.0, 3.0, 4.0)]

    def face(indices):
        loop = f.create_entity("IfcPolyLoop", Polygon=[points[i] for i in indices])
        bound = f.create_entity("IfcFaceOuterBound", Bound=loop, Orientation=True)
        return f.create_entity("IfcFace", Bounds=[bound])

    faces = [face(indices) for indices in BOX_FACES]
    # The OfficeBuilding shape: two points, each listed twice, along an
    # existing box edge. It encloses no area.
    faces.append(face((4, 4, 5, 5)))
    shell = f.create_entity("IfcClosedShell", CfsFaces=faces)
    return [f.create_entity("IfcFacetedBrep", Outer=shell)], "Brep"


def polygonal_face_set_quads(f):
    """kernel#160: a closed cube as an IfcPolygonalFaceSet of quads."""
    corners = box_corners(2.0, 2.0, 2.0)
    coords = f.create_entity("IfcCartesianPointList3D",
                             CoordList=[[float(v) * MM for v in c] for c in corners])
    faces = [f.create_entity("IfcIndexedPolygonalFace",
                             CoordIndex=[i + 1 for i in indices])
             for indices in BOX_FACES]
    face_set = f.create_entity("IfcPolygonalFaceSet", Coordinates=coords, Closed=True,
                               Faces=faces)
    return [face_set], "Tessellation"


def shell_based_surface_model(f):
    """kernel#161: two triangles in an OPEN shell; a surface, never a solid."""
    a, b = pt(f, (0.0, 0.0, 0.0)), pt(f, (0.4, 0.0, 0.0))
    c, d = pt(f, (0.4, 0.3, 0.0)), pt(f, (0.0, 0.3, 0.0))

    def tri(p, q, r):
        loop = f.create_entity("IfcPolyLoop", Polygon=[p, q, r])
        bound = f.create_entity("IfcFaceOuterBound", Bound=loop, Orientation=True)
        return f.create_entity("IfcFace", Bounds=[bound])

    shell = f.create_entity("IfcOpenShell", CfsFaces=[tri(a, b, c), tri(a, c, d)])
    return [f.create_entity("IfcShellBasedSurfaceModel", SbsmBoundary=[shell])], "SurfaceModel"


def wall_with_opening(f, body, storey_place):
    """#44: a wall voided by a flush door whose body is extruded downward."""
    wall = product(f, "IfcWall", "wall-with-opening", body,
                   [extrusion(f, rectangle(f, 4.0, 0.3, centre=(2.0, 0.15)), 3.0)],
                   "SweptSolid", storey_place)
    # The door body hangs from its lintel at z = 2 m and extrudes 2 m down.
    door_body = extrusion(f, rectangle(f, 1.0, 0.3, centre=(1.5, 0.15)), 2.0,
                          position=placement(f, (0.0, 0.0, 2.0)),
                          direction=(0.0, 0.0, -1.0))
    opening = f.create_entity(
        "IfcOpeningElement", GlobalId=guid("door-opening"), Name="door-opening",
        ObjectPlacement=local(f, wall.ObjectPlacement),
        Representation=shape(f, body, [door_body], "SweptSolid"))
    f.create_entity("IfcRelVoidsElement", GlobalId=guid("wall-voids-door"),
                    RelatingBuildingElement=wall, RelatedOpeningElement=opening)
    return wall


SOLO = [
    ("composite-curve-profile", composite_curve_profile),
    ("bounded-half-space-clip", bounded_half_space_clip),
    ("collapsed-poly-loop", collapsed_poly_loop),
    ("polygonal-face-set-quads", polygonal_face_set_quads),
    ("shell-based-surface-model", shell_based_surface_model),
]


def build():
    f = ifcopenshell.file(schema=SCHEMA)
    assignment, ctx, body = units_and_context(f)
    project = f.create_entity("IfcProject", GlobalId=guid("project"),
                              Name="#47 meshing coverage", UnitsInContext=assignment,
                              RepresentationContexts=[ctx])
    site_place = local(f)
    building_place = local(f, site_place)
    storey_place = local(f, building_place)
    site = f.create_entity("IfcSite", GlobalId=guid("site"), Name="Site",
                           ObjectPlacement=site_place)
    building = f.create_entity("IfcBuilding", GlobalId=guid("building"), Name="Building",
                               ObjectPlacement=building_place)
    storey = f.create_entity("IfcBuildingStorey", GlobalId=guid("storey"), Name="Storey",
                             ObjectPlacement=storey_place)
    for parent, child in [(project, site), (site, building), (building, storey)]:
        f.create_entity("IfcRelAggregates", GlobalId=guid(f"aggregates-{child.Name}"),
                        RelatingObject=parent, RelatedObjects=[child])

    products = [wall_with_opening(f, body, storey_place)]
    for name, builder in SOLO:
        items, rep_type = builder(f)
        products.append(product(f, "IfcBuildingElementProxy", name, body, items,
                                rep_type, storey_place))
    f.create_entity("IfcRelContainedInSpatialStructure", GlobalId=guid("contained"),
                    RelatingStructure=storey, RelatedElements=products)
    return f


def main():
    if len(sys.argv) != 2:
        print("usage: gen_coverage_fixtures.py <outdir>", file=sys.stderr)
        return 2
    out = pathlib.Path(sys.argv[1])
    out.mkdir(parents=True, exist_ok=True)
    model = build()
    # A fixed timestamp keeps regeneration byte-stable.
    model.header.file_name.time_stamp = "1970-01-01T00:00:00"
    target = out / NAME
    model.write(str(target))
    print(f"{target.stat().st_size:>7} {NAME}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
