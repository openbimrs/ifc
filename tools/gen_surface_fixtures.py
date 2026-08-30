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


FIXTURES = [
    ("synthetic_elementary_surfaces.ifc", elementary_surfaces),
    ("synthetic_surface_of_revolution.ifc", surface_of_revolution),
    ("synthetic_bspline_surface.ifc", bspline_surface),
    ("synthetic_curve_bounded_plane.ifc", curve_bounded_plane),
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
