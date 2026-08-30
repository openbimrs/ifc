#!/usr/bin/env python3
"""Generate minimal IFC4 surface fixtures with ifcopenshell.

No licensed public corpus we surveyed (ifc-lite MPL-2.0, buildingSMART
CC-BY-4.0, 909 files) contains curved or B-spline surfaces, so these are
generated rather than sourced. Output is our own data: no licence question.

Run:  python3 tools/gen_surface_fixtures.py <outdir>
"""

import sys
import pathlib
import ifcopenshell

SCHEMA = "IFC4"


def new_model():
    f = ifcopenshell.file(schema=SCHEMA)
    return f


def pt(f, xyz):
    return f.create_entity("IfcCartesianPoint", Coordinates=[float(v) for v in xyz])


def dr(f, xyz):
    return f.create_entity("IfcDirection", DirectionRatios=[float(v) for v in xyz])


def placement(f, origin=(0.0, 0.0, 0.0), axis=None, ref=None):
    kw = {"Location": pt(f, origin)}
    if axis is not None:
        kw["Axis"] = dr(f, axis)
    if ref is not None:
        kw["RefDirection"] = dr(f, ref)
    return f.create_entity("IfcAxis2Placement3D", **kw)


def units_and_context(f):
    """Millimetre length + degree angle: the combination real exporters emit.

    Degrees matter here. A conic trim parameter is an angle, so a fixture in
    degrees exercises the plane-angle conversion that a radians-only file
    silently passes.
    """
    mm = f.create_entity("IfcSIUnit", UnitType="LENGTHUNIT", Prefix="MILLI", Name="METRE")
    rad = f.create_entity("IfcSIUnit", UnitType="PLANEANGLEUNIT", Name="RADIAN")
    deg_ratio = f.create_entity("IfcMeasureWithUnit",
                                ValueComponent=f.create_entity("IfcPlaneAngleMeasure", 0.017453292519943295),
                                UnitComponent=rad)
    deg = f.create_entity("IfcConversionBasedUnit",
                          Dimensions=f.create_entity("IfcDimensionalExponents",
                                                     0, 0, 0, 0, 0, 0, 0),
                          UnitType="PLANEANGLEUNIT",
                          Name="DEGREE",
                          ConversionFactor=deg_ratio)
    assignment = f.create_entity("IfcUnitAssignment", Units=[mm, deg])
    ctx = f.create_entity("IfcGeometricRepresentationContext",
                          ContextType="Model",
                          CoordinateSpaceDimension=3,
                          Precision=1e-5,
                          WorldCoordinateSystem=placement(f),
                          TrueNorth=dr(f, (0.0, 1.0, 0.0)))
    return assignment, ctx


def wrap(f, ctx, items, rep_id):
    """Attach items to a product so the file is a real model, not loose geometry.

    A bare IfcCylindricalSurface with no product would parse but would be
    unreachable, which our own corpus gate reports as a finding. Every
    generated fixture therefore carries project/site/building/storey and a
    contained proxy, exactly as an exporter would emit.
    """
    shape = f.create_entity("IfcShapeRepresentation",
                            ContextOfItems=ctx,
                            RepresentationIdentifier="Body",
                            RepresentationType=rep_id,
                            Items=items)
    return f.create_entity("IfcProductDefinitionShape", Representations=[shape])


def spatial_tree(f, assignment, ctx, product):
    proj = f.create_entity("IfcProject", GlobalId=ifcopenshell.guid.new(),
                           Name="synthetic surface fixture",
                           UnitsInContext=assignment,
                           RepresentationContexts=[ctx])
    site = f.create_entity("IfcSite", GlobalId=ifcopenshell.guid.new(), Name="Site",
                           ObjectPlacement=f.create_entity("IfcLocalPlacement",
                                                           RelativePlacement=placement(f)))
    bldg = f.create_entity("IfcBuilding", GlobalId=ifcopenshell.guid.new(), Name="Building",
                           ObjectPlacement=f.create_entity("IfcLocalPlacement",
                                                           RelativePlacement=placement(f)))
    storey = f.create_entity("IfcBuildingStorey", GlobalId=ifcopenshell.guid.new(), Name="Storey",
                             ObjectPlacement=f.create_entity("IfcLocalPlacement",
                                                             RelativePlacement=placement(f)))
    f.create_entity("IfcRelAggregates", GlobalId=ifcopenshell.guid.new(),
                    RelatingObject=proj, RelatedObjects=[site])
    f.create_entity("IfcRelAggregates", GlobalId=ifcopenshell.guid.new(),
                    RelatingObject=site, RelatedObjects=[bldg])
    f.create_entity("IfcRelAggregates", GlobalId=ifcopenshell.guid.new(),
                    RelatingObject=bldg, RelatedObjects=[storey])
    f.create_entity("IfcRelContainedInSpatialStructure", GlobalId=ifcopenshell.guid.new(),
                    RelatingStructure=storey, RelatedElements=[product])
    return proj


def proxy(f, shape):
    return f.create_entity("IfcBuildingElementProxy",
                           GlobalId=ifcopenshell.guid.new(),
                           Name="surface carrier",
                           ObjectPlacement=f.create_entity("IfcLocalPlacement",
                                                           RelativePlacement=placement(f)),
                           Representation=shape)


def build(fn):
    """Run one builder into a fresh file with the shared scaffold."""
    f = new_model()
    assignment, ctx = units_and_context(f)
    items, rep_id = fn(f)
    shape = wrap(f, ctx, items, rep_id)
    product = proxy(f, shape)
    spatial_tree(f, assignment, ctx, product)
    return f


def elementary_surfaces(f):
    """Cylinder, sphere, torus: the three curved elementary families.

    Each sits on a DIFFERENT placement with a non-default axis. A fixture
    with everything at the identity frame cannot tell a correct frame from a
    dropped one, which is the whole point of committing it.
    """
    cyl = f.create_entity("IfcCylindricalSurface",
                          Position=placement(f, (100.0, 0.0, 0.0), (0.0, 0.0, 1.0), (1.0, 0.0, 0.0)),
                          Radius=250.0)
    sph = f.create_entity("IfcSphericalSurface",
                          Position=placement(f, (0.0, 400.0, 0.0), (0.0, 1.0, 0.0), (1.0, 0.0, 0.0)),
                          Radius=180.0)
    tor = f.create_entity("IfcToroidalSurface",
                          Position=placement(f, (0.0, 0.0, 700.0), (1.0, 0.0, 0.0), (0.0, 1.0, 0.0)),
                          MajorRadius=300.0,
                          MinorRadius=60.0)
    return [cyl, sph, tor], "SurfaceModel"


def surface_of_revolution(f):
    """A profile curve revolved about an axis, plus a rectangular trim.

    IfcSurfaceOfRevolution takes an AxisPosition (IfcAxis1Placement), NOT the
    IfcAxis2Placement3D every other family uses. Mixing them up is a silent
    error: both carry a Location, so a wrong reader still produces a surface.
    """
    profile = f.create_entity("IfcPolyline", Points=[
        pt(f, (200.0, 0.0, 0.0)),
        pt(f, (260.0, 0.0, 150.0)),
        pt(f, (220.0, 0.0, 320.0)),
    ])
    # Deliberately OFF-ORIGIN: an axis at (0,0,0) makes a missing millimetre
    # conversion unobservable, because scaling zero is still zero.
    axis = f.create_entity("IfcAxis1Placement",
                           Location=pt(f, (40.0, 0.0, 25.0)),
                           Axis=dr(f, (0.0, 0.0, 1.0)))
    rev = f.create_entity("IfcSurfaceOfRevolution",
                          SweptCurve=f.create_entity("IfcArbitraryOpenProfileDef",
                                                     ProfileType="CURVE",
                                                     Curve=profile),
                          AxisPosition=axis)
    # Trim in DEGREES: u is the revolution angle, v is along the profile.
    trimmed = f.create_entity("IfcRectangularTrimmedSurface",
                              BasisSurface=rev,
                              U1=0.0, V1=0.0, U2=90.0, V2=1.0,
                              Usense=True, Vsense=True)
    return [rev, trimmed], "SurfaceModel"


def bspline_surface(f):
    """A bicubic-in-u, linear-in-v B-spline patch with explicit knots.

    Degrees differ per direction (3 and 1) on purpose: a reader that assumes a
    single degree, or that transposes the control net, produces a plausible
    surface from wrong data. The knot multiplicities (4,4) and (2,2) are the
    clamped form, so the patch interpolates its corner points.
    """
    grid = []
    for i, u in enumerate((0.0, 100.0, 200.0, 300.0)):
        row = []
        for j, v in enumerate((0.0, 400.0)):
            # A saddle: height depends on both directions, so a transposed
            # control net is geometrically detectable, not just cosmetic.
            z = 60.0 if (i in (1, 2)) != (j == 1) else 0.0
            row.append(pt(f, (u, v, z)))
        grid.append(row)
    return [f.create_entity("IfcBSplineSurfaceWithKnots",
                            UDegree=3,
                            VDegree=1,
                            ControlPointsList=grid,
                            SurfaceForm="UNSPECIFIED",
                            UClosed=False,
                            VClosed=False,
                            SelfIntersect=False,
                            UMultiplicities=[4, 4],
                            VMultiplicities=[2, 2],
                            UKnots=[0.0, 1.0],
                            VKnots=[0.0, 1.0],
                            KnotSpec="UNSPECIFIED")], "SurfaceModel"


def curve_bounded_plane(f):
    """A plane restricted by an outer boundary and one inner hole."""
    base = f.create_entity("IfcPlane",
                           Position=placement(f, (0.0, 0.0, 50.0), (0.0, 0.0, 1.0), (1.0, 0.0, 0.0)))
    outer = f.create_entity("IfcPolyline", Points=[
        pt(f, (0.0, 0.0, 0.0)), pt(f, (500.0, 0.0, 0.0)),
        pt(f, (500.0, 300.0, 0.0)), pt(f, (0.0, 300.0, 0.0)),
        pt(f, (0.0, 0.0, 0.0)),
    ])
    hole = f.create_entity("IfcPolyline", Points=[
        pt(f, (150.0, 100.0, 0.0)), pt(f, (250.0, 100.0, 0.0)),
        pt(f, (250.0, 200.0, 0.0)), pt(f, (150.0, 200.0, 0.0)),
        pt(f, (150.0, 100.0, 0.0)),
    ])
    return [f.create_entity("IfcCurveBoundedPlane",
                            BasisSurface=base,
                            OuterBoundary=outer,
                            InnerBoundaries=[hole])], "SurfaceModel"



def vp(f, xyz):
    return f.create_entity("IfcVertexPoint", VertexGeometry=pt(f, xyz))


def ec(f, a, b, curve, same=True):
    return f.create_entity("IfcEdgeCurve", EdgeStart=a, EdgeEnd=b,
                            EdgeGeometry=curve, SameSense=same)


def oe(f, edge, orient=True):
    return f.create_entity("IfcOrientedEdge", EdgeElement=edge, Orientation=orient)


def advanced_brep(f):
    # A half-cylinder plug: two planar ends, one curved lateral face.
    # Curved faces are the whole point: Face.surface must be filled.
    r, h = 120.0, 200.0
    axis = placement(f, (0.0, 0.0, 0.0), (0.0, 0.0, 1.0), (1.0, 0.0, 0.0))
    cyl = f.create_entity("IfcCylindricalSurface", Position=axis, Radius=r)
    # Four vertices: two on the bottom rim, two on the top rim.
    v_b0, v_b1 = vp(f, (r, 0.0, 0.0)), vp(f, (-r, 0.0, 0.0))
    v_t0, v_t1 = vp(f, (r, 0.0, h)), vp(f, (-r, 0.0, h))
    # Rim arcs are real circles, so the edge carries an exact curve.
    bot_c = f.create_entity("IfcCircle", Position=axis, Radius=r)
    top_axis = placement(f, (0.0, 0.0, h), (0.0, 0.0, 1.0), (1.0, 0.0, 0.0))
    top_c = f.create_entity("IfcCircle", Position=top_axis, Radius=r)
    # Vertical seams are straight: IfcLine with a unit-magnitude vector.
    def vline(x):
        d = f.create_entity("IfcVector", Orientation=dr(f, (0.0, 0.0, 1.0)), Magnitude=1.0)
        return f.create_entity("IfcLine", Pnt=pt(f, (x, 0.0, 0.0)), Dir=d)
    e_bot = ec(f, v_b0, v_b1, bot_c)
    e_top = ec(f, v_t0, v_t1, top_c)
    e_s0 = ec(f, v_b0, v_t0, vline(r))
    # Authored against its curve so the flag is observable.
    e_s1 = ec(f, v_b1, v_t1, vline(-r), same=False)
    # The curved lateral face. Its edge loop mixes forward and reversed
    # uses of SHARED edges -- that sharing is what makes it a solid.
    lat_loop = f.create_entity("IfcEdgeLoop", EdgeList=[
        oe(f, e_bot, True), oe(f, e_s1, True),
        oe(f, e_top, False), oe(f, e_s0, False)])
    lat_bound = f.create_entity("IfcFaceOuterBound", Bound=lat_loop, Orientation=True)
    lat = f.create_entity("IfcAdvancedFace", Bounds=[lat_bound],
                          FaceSurface=cyl, SameSense=True)
    # Planar caps. The bottom uses SameSense=False so a lowerer that
    # ignores the flag produces an inside-out face and nothing complains.
    bot_pl = f.create_entity("IfcPlane", Position=axis)
    top_pl = f.create_entity("IfcPlane", Position=top_axis)
    bot_loop = f.create_entity("IfcEdgeLoop", EdgeList=[
        oe(f, e_bot, True), oe(f, e_bot, False)])
    top_loop = f.create_entity("IfcEdgeLoop", EdgeList=[
        oe(f, e_top, True), oe(f, e_top, False)])
    bot_b = f.create_entity("IfcFaceOuterBound", Bound=bot_loop, Orientation=True)
    top_b = f.create_entity("IfcFaceOuterBound", Bound=top_loop, Orientation=True)
    bot_f = f.create_entity("IfcAdvancedFace", Bounds=[bot_b],
                            FaceSurface=bot_pl, SameSense=False)
    top_f = f.create_entity("IfcAdvancedFace", Bounds=[top_b],
                            FaceSurface=top_pl, SameSense=True)
    shell = f.create_entity("IfcClosedShell", CfsFaces=[lat, bot_f, top_f])
    solid = f.create_entity("IfcAdvancedBrep", Outer=shell)
    return [solid], "AdvancedBrep"

def misc_items(f):
    # A pyramid and a rotated bounding box. The box is placed on a
    # 45-degree rotated context so a naive min/max passthrough is wrong.
    pyr = f.create_entity("IfcRectangularPyramid",
                          Position=placement(f, (0.0, 0.0, 0.0)),
                          XLength=300.0, YLength=200.0, Height=450.0)
    box = f.create_entity("IfcBoundingBox", Corner=pt(f, (10.0, 20.0, 30.0)),
                          XDim=100.0, YDim=200.0, ZDim=300.0)
    return [pyr, box], "CSG"


def collections(f):
    # A geometric curve set (polyline + circle) and a shell-based
    # surface model built from one open shell of two triangles.
    poly = f.create_entity("IfcPolyline", Points=[pt(f, (0.0, 0.0, 0.0)),
                                                pt(f, (500.0, 0.0, 0.0)),
                                                pt(f, (500.0, 400.0, 0.0))])
    circ = f.create_entity("IfcCircle",
                           Position=placement(f, (100.0, 100.0, 0.0)),
                           Radius=75.0)
    cset = f.create_entity("IfcGeometricCurveSet", Elements=[poly, circ])
    # Two triangles sharing an edge, as an OPEN shell: a surface model,
    # not a solid. Nothing here should acquire a volume.
    a, b = pt(f, (0.0, 0.0, 0.0)), pt(f, (400.0, 0.0, 0.0))
    cc, d = pt(f, (400.0, 300.0, 0.0)), pt(f, (0.0, 300.0, 0.0))
    def tri(p, q, r_):
        lp = f.create_entity("IfcPolyLoop", Polygon=[p, q, r_])
        bd = f.create_entity("IfcFaceOuterBound", Bound=lp, Orientation=True)
        return f.create_entity("IfcFace", Bounds=[bd])
    sh = f.create_entity("IfcOpenShell", CfsFaces=[tri(a, b, cc), tri(a, cc, d)])
    sbsm = f.create_entity("IfcShellBasedSurfaceModel", SbsmBoundary=[sh])
    return [cset, sbsm], "GeometricCurveSet"


def tapered_sweeps(f):
    """The tapered and variable-section sweep families.

    Every profile pair here is DISTINCT in size. A lowerer that reuses
    SweptArea for both ends of a taper produces a prism: geometry that builds,
    renders and is wrong. Distinct sizes make that observable.
    """
    # Tapered linear extrusion: 400x300 mm down to 200x150 mm.
    start = f.create_entity("IfcRectangleProfileDef", ProfileType="AREA",
                            XDim=400.0, YDim=300.0)
    end = f.create_entity("IfcRectangleProfileDef", ProfileType="AREA",
                          XDim=200.0, YDim=150.0)
    tap_ext = f.create_entity("IfcExtrudedAreaSolidTapered",
                              SweptArea=start, Position=placement(f, (0.0, 0.0, 0.0)),
                              ExtrudedDirection=dr(f, (0.0, 0.0, 1.0)),
                              Depth=2500.0, EndSweptArea=end)

    # Tapered revolution. Angle is authored in DEGREES, as this file declares,
    # so a missing unit conversion turns 90 into fourteen full turns.
    rstart = f.create_entity("IfcCircleProfileDef", ProfileType="AREA", Radius=120.0)
    rend = f.create_entity("IfcCircleProfileDef", ProfileType="AREA", Radius=60.0)
    axis = f.create_entity("IfcAxis1Placement", Location=pt(f, (900.0, 0.0, 0.0)),
                           Axis=dr(f, (0.0, 1.0, 0.0)))
    tap_rev = f.create_entity("IfcRevolvedAreaSolidTapered",
                              SweptArea=rstart, Position=placement(f, (0.0, 0.0, 0.0)),
                              Axis=axis, Angle=90.0, EndSweptArea=rend)

    # Fixed-reference sweep. The reference is +Z, not the +X a lowerer would
    # reach for by default, so dropping or hardcoding it is observable.
    fr_prof = f.create_entity("IfcRectangleProfileDef", ProfileType="AREA",
                              XDim=150.0, YDim=80.0)
    path = f.create_entity("IfcPolyline", Points=[pt(f, (0.0, 0.0, 0.0)),
                                                 pt(f, (1000.0, 0.0, 0.0)),
                                                 pt(f, (1000.0, 800.0, 0.0))])
    fixed = f.create_entity("IfcFixedReferenceSweptAreaSolid",
                            SweptArea=fr_prof, Position=placement(f, (0.0, 0.0, 0.0)),
                            Directrix=path, StartParam=0.0, EndParam=2.0,
                            FixedReference=dr(f, (0.0, 0.0, 1.0)))

    # Polygonal disk WITHOUT a fillet: sharp corners, which the neutral
    # SweptDisk models exactly. This one must lower.
    poly_path = f.create_entity("IfcPolyline", Points=[pt(f, (0.0, 0.0, 0.0)),
                                                      pt(f, (600.0, 0.0, 0.0)),
                                                      pt(f, (600.0, 500.0, 0.0))])
    sharp = f.create_entity("IfcSweptDiskSolidPolygonal",
                            Directrix=poly_path, Radius=45.0, InnerRadius=38.0,
                            StartParam=0.0, EndParam=2.0)

    # WITH a fillet: rounded corners the neutral SweptDisk cannot express.
    # Lowering this anyway would silently sharpen every bend in a pipe run.
    filleted = f.create_entity("IfcSweptDiskSolidPolygonal",
                               Directrix=poly_path, Radius=45.0,
                               StartParam=0.0, EndParam=2.0, FilletRadius=90.0)

    return [tap_ext, tap_rev, fixed, sharp, filleted], "SweptSolid"


def sectioned_spine(f):
    """Cross sections positioned along a composite curve.

    The three sections are DIFFERENT sizes and the three positions are
    DISTINCT, so a lowerer that keeps only the first section, or collapses the
    placements, is observable.
    """
    secs = [f.create_entity("IfcRectangleProfileDef", ProfileType="AREA",
                            XDim=x, YDim=y)
            for x, y in ((300.0, 200.0), (240.0, 160.0), (180.0, 120.0))]

    # The schema requires an IfcCompositeCurve, not a bare polyline.
    seg1 = f.create_entity("IfcPolyline", Points=[pt(f, (0.0, 0.0, 0.0)),
                                                 pt(f, (1200.0, 0.0, 0.0))])
    seg2 = f.create_entity("IfcPolyline", Points=[pt(f, (1200.0, 0.0, 0.0)),
                                                  pt(f, (1200.0, 900.0, 0.0))])
    segments = [f.create_entity("IfcCompositeCurveSegment", Transition="CONTINUOUS",
                                SameSense=True, ParentCurve=s)
                for s in (seg1, seg2)]
    spine = f.create_entity("IfcCompositeCurve", Segments=segments, SelfIntersect=False)

    positions = [placement(f, p) for p in ((0.0, 0.0, 0.0),
                                           (1200.0, 0.0, 0.0),
                                           (1200.0, 900.0, 0.0))]
    item = f.create_entity("IfcSectionedSpine", SpineCurve=spine,
                           CrossSections=secs, CrossSectionPositions=positions)
    return [item], "AdvancedSweptSolid"




def steel_profiles(f):
    """Every profile family this slice lowers, with observable dimensions.

    No committed fixture carried a steel section before this, which is exactly
    how 13 profile families stayed unimplemented while the census reported
    full coverage. Dimensions are realistic (an HEA300-like I, an L100x100x8)
    so a wrong lowering looks wrong to anyone who knows sections.
    """
    prof = []

    # Symmetric I with fillet AND flange edge radius: dropping either was the
    # old lossy behaviour, and both are separately observable.
    prof.append(f.create_entity(
        "IfcIShapeProfileDef", ProfileType="AREA", ProfileName="HEA300-like",
        OverallWidth=300.0, OverallDepth=290.0, WebThickness=8.5,
        FlangeThickness=14.0, FilletRadius=27.0, FlangeEdgeRadius=3.0,
        # An explicit flange slope in DEGREES, as the file's unit assignment
        # declares. A lowerer that scales this by the LENGTH factor turns a
        # 2 degree taper into millimetres-times-radians, which no assertion
        # on lengths alone would notice.
        FlangeSlope=2.0))

    # Asymmetric I: top flange DELIBERATELY narrower than the bottom. A
    # lowerer collapsing this onto the symmetric variant must pick one width,
    # and the fixture makes that choice visible either way.
    prof.append(f.create_entity(
        "IfcAsymmetricIShapeProfileDef", ProfileType="AREA", ProfileName="asym-I",
        BottomFlangeWidth=300.0, OverallDepth=290.0, WebThickness=8.5,
        BottomFlangeThickness=14.0, BottomFlangeFilletRadius=27.0,
        TopFlangeWidth=200.0, TopFlangeThickness=12.0, TopFlangeFilletRadius=21.0))

    # L with UNEQUAL legs: Width is optional and defaults to Depth, so an
    # equal-leg fixture could not catch a lowerer that ignores Width.
    prof.append(f.create_entity(
        "IfcLShapeProfileDef", ProfileType="AREA", ProfileName="L150x100x10",
        Depth=150.0, Width=100.0, Thickness=10.0,
        FilletRadius=12.0, EdgeRadius=6.0))

    prof.append(f.create_entity(
        "IfcTShapeProfileDef", ProfileType="AREA", ProfileName="T200x200x12",
        Depth=200.0, FlangeWidth=200.0, WebThickness=12.0,
        FlangeThickness=15.0, FilletRadius=18.0, FlangeEdgeRadius=4.0,
        WebEdgeRadius=3.0))

    prof.append(f.create_entity(
        "IfcUShapeProfileDef", ProfileType="AREA", ProfileName="UPN200",
        Depth=200.0, FlangeWidth=75.0, WebThickness=8.5,
        FlangeThickness=11.5, FilletRadius=11.5, EdgeRadius=6.0))

    # C section: girth is the returned lip, the dimension that distinguishes
    # a lipped channel from a plain one.
    prof.append(f.create_entity(
        "IfcCShapeProfileDef", ProfileType="AREA", ProfileName="C200x75x20",
        Depth=200.0, Width=75.0, WallThickness=2.5, Girth=20.0,
        InternalFilletRadius=5.0))

    prof.append(f.create_entity(
        "IfcZShapeProfileDef", ProfileType="AREA", ProfileName="Z200x75",
        Depth=200.0, FlangeWidth=75.0, WebThickness=2.5,
        FlangeThickness=2.5, FilletRadius=5.0, EdgeRadius=3.0))

    # Ellipse: distinct semi-axes so a swapped or averaged pair is visible.
    prof.append(f.create_entity(
        "IfcEllipseProfileDef", ProfileType="AREA", ProfileName="ellipse",
        SemiAxis1=200.0, SemiAxis2=120.0))

    # Trapezium with a NEGATIVE top offset: the one profile dimension that is
    # a plain IfcLengthMeasure and may legitimately be below zero.
    prof.append(f.create_entity(
        "IfcTrapeziumProfileDef", ProfileType="AREA", ProfileName="trapezium",
        BottomXDim=300.0, TopXDim=180.0, YDim=150.0, TopXOffset=-40.0))

    return prof


def derived_profiles(f, base_i):
    """Composite, derived and mirrored profiles, which nest other profiles."""
    # Derived: a NON-TRIVIAL operator. Scale 2.0 plus a translation, so a
    # lowerer that drops the operator or applies identity is caught.
    op = f.create_entity(
        "IfcCartesianTransformationOperator2D",
        LocalOrigin=f.create_entity("IfcCartesianPoint", Coordinates=[50.0, 25.0]),
        Scale=2.0)
    derived = f.create_entity(
        "IfcDerivedProfileDef", ProfileType="AREA", ProfileName="derived-2x",
        ParentProfile=base_i, Operator=op, Label="scaled")

    # Mirrored: Operator is DERIVED in the schema, so a file cannot carry one.
    # The mirror about the local y axis is implied by the TYPE alone, which is
    # exactly why lowering this through the IfcDerivedProfileDef path would
    # read a null operator and silently produce an unmirrored profile.
    mirrored = f.create_entity(
        "IfcMirroredProfileDef", ProfileType="AREA", ProfileName="mirrored-L",
        ParentProfile=base_i, Label="mirrored")

    # Composite of two DIFFERENT profiles, so dropping a member is visible.
    c1 = f.create_entity(
        "IfcRectangleProfileDef", ProfileType="AREA", ProfileName="comp-rect",
        XDim=200.0, YDim=100.0)
    c2 = f.create_entity(
        "IfcCircleProfileDef", ProfileType="AREA", ProfileName="comp-circle",
        Radius=60.0)
    composite = f.create_entity(
        "IfcCompositeProfileDef", ProfileType="AREA", ProfileName="composite",
        Profiles=[c1, c2], Label="two-part")

    # Centre-line: an OPEN curve plus a thickness. Its parent
    # IfcArbitraryOpenProfileDef is not an area at all, but adding a wall
    # thickness to a centre line DOES sweep a closed region.
    line = f.create_entity("IfcPolyline", Points=[
        f.create_entity("IfcCartesianPoint", Coordinates=[0.0, 0.0]),
        f.create_entity("IfcCartesianPoint", Coordinates=[300.0, 0.0]),
        f.create_entity("IfcCartesianPoint", Coordinates=[300.0, 200.0])])
    # NOT returned for extrusion: this family is declared unlowered, and a
    # solid built on it would fail the corpus dispatch gate on
    # IfcExtrudedAreaSolid, masking real regressions in that family. It stays
    # in the file as a free-standing record so the refusal path has real data.
    f.create_entity(
        "IfcCenterLineProfileDef", ProfileType="AREA", ProfileName="centerline",
        Curve=line, Thickness=8.0)

    return [derived, mirrored, composite]


def profile_families(f):
    """Extrude every profile family this slice lowers.

    A profile only reaches the lowerer through a solid that references it, so
    each one backs a short extrusion.
    """
    prof = steel_profiles(f)
    prof += derived_profiles(f, prof[0])

    solids = []
    for i, p in enumerate(prof):
        place = f.create_entity(
            "IfcAxis2Placement3D",
            Location=pt(f, (float(i) * 500.0, 0.0, 0.0)))
        solids.append(f.create_entity(
            "IfcExtrudedAreaSolid", SweptArea=p, Position=place,
            ExtrudedDirection=dr(f, (0.0, 0.0, 1.0)), Depth=1000.0))
    return solids, "SweptSolid"


FIXTURES = [
    ("synthetic_profile_families.ifc", profile_families),
    ("synthetic_elementary_surfaces.ifc", elementary_surfaces),
    ("synthetic_surface_of_revolution.ifc", surface_of_revolution),
    ("synthetic_bspline_surface.ifc", bspline_surface),
    ("synthetic_advanced_brep.ifc", advanced_brep),
    ("synthetic_primitives_and_bbox.ifc", misc_items),
    ("synthetic_collections.ifc", collections),
    ("synthetic_curve_bounded_plane.ifc", curve_bounded_plane),
    ("synthetic_tapered_sweeps.ifc", tapered_sweeps),
    ("synthetic_sectioned_spine.ifc", sectioned_spine),
]


def main():
    if len(sys.argv) != 2:
        print("usage: gen_surface_fixtures.py <outdir>", file=sys.stderr)
        return 2
    out = pathlib.Path(sys.argv[1])
    out.mkdir(parents=True, exist_ok=True)
    for name, fn in FIXTURES:
        model = build(fn)
        target = out / name
        model.write(str(target))
        print(f"{target.stat().st_size:>7} {name}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
