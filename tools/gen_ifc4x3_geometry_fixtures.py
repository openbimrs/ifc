#!/usr/bin/env python3
"""Generate the IFC4X3 ADD2 geometry-family fixture with ifcopenshell.

`crates/ifc-geometry/tests/lower_dispatch_corpus.rs` requires a committed
corpus instance for every `dispatch::PLANNED` family and for every
`IMPLEMENTED` family, so the IFC4X3-only representation items need a file.
No licence-clean public corpus we know of carries all of them. Output is our
own data, so there is no licence question.

The file carries one valid instance of each IFC4X3-only family that
`ifc-geometry` either lowers or declares in `dispatch::PLANNED` (#243):

- the six `IfcSpiral` subtypes and `IfcPolynomialCurve`, each as the parent
  curve of an `IfcCurveSegment`;
- a horizontal `IfcCompositeCurve`, an `IfcGradientCurve` over it, and an
  `IfcSegmentedReferenceCurve` over that;
- `IfcDirectrixDerivedReferenceSweptAreaSolid`,
  `IfcSectionedSolidHorizontal` and `IfcSectionedSurface` (with
  `IfcOpenCrossProfileDef` sections) along the gradient curve;
- `IfcOffsetCurveByDistances` with `IfcPointByDistanceExpression` offsets;
- an `IfcTriangulatedIrregularNetwork` whose flags are non-negative (one
  breakline, no void or hole), the admitted form.

Every standalone composite curve has exactly one segment whose transition is
DISCONTINUOUS: the schema's `CurveContinuous` rule requires exactly one
discontinuous segment on an open curve, and a single segment cannot claim a
continuity its neighbours do not have.

Metres and radians, as alignment exporters write them. Entity ids, GUIDs and
the header timestamp are fixed, so regeneration is byte-stable.

Run:  python3 tools/gen_ifc4x3_geometry_fixtures.py test/fixtures/synthetic-surfaces
"""

import pathlib
import sys
import uuid

import ifcopenshell
import ifcopenshell.guid

SCHEMA = "IFC4X3_ADD2"
NAME = "synthetic_ifc4x3_geometry_families.ifc"


def guid(label):
    """A GUID that is stable across regenerations."""
    return ifcopenshell.guid.compress(
        uuid.uuid5(uuid.NAMESPACE_URL, "openbimrs/ifc#243/" + label).hex)


def pt(f, xyz):
    return f.create_entity("IfcCartesianPoint", Coordinates=[float(v) for v in xyz])


def dr(f, xyz):
    return f.create_entity("IfcDirection", DirectionRatios=[float(v) for v in xyz])


def length(f, value):
    return f.create_entity("IfcLengthMeasure", float(value))


def place2(f, origin=(0.0, 0.0), ref=(1.0, 0.0)):
    return f.create_entity("IfcAxis2Placement2D", Location=pt(f, origin),
                           RefDirection=dr(f, ref))


def place3(f, origin=(0.0, 0.0, 0.0)):
    return f.create_entity("IfcAxis2Placement3D", Location=pt(f, origin))


def segment(f, parent, run, transition="DISCONTINUOUS", placement=None):
    """An `IfcCurveSegment` covering `run` metres of `parent` from its start."""
    return f.create_entity(
        "IfcCurveSegment",
        Transition=transition,
        Placement=placement if placement is not None else place2(f),
        SegmentStart=length(f, 0.0),
        SegmentLength=length(f, run),
        ParentCurve=parent)


def composite(f, segments):
    return f.create_entity("IfcCompositeCurve", Segments=segments, SelfIntersect=False)


def spirals(f):
    """One composite curve per spiral family, each a single 50 m segment."""
    p = place2
    parents = [
        f.create_entity("IfcClothoid", Position=p(f), ClothoidConstant=100.0),
        f.create_entity("IfcCosineSpiral", Position=p(f), CosineTerm=200.0,
                        ConstantTerm=400.0),
        f.create_entity("IfcSineSpiral", Position=p(f), SineTerm=200.0,
                        LinearTerm=300.0),
        f.create_entity("IfcSecondOrderPolynomialSpiral", Position=p(f),
                        QuadraticTerm=150.0, LinearTerm=200.0),
        f.create_entity("IfcThirdOrderPolynomialSpiral", Position=p(f),
                        CubicTerm=120.0),
        f.create_entity("IfcSeventhOrderPolynomialSpiral", Position=p(f),
                        SepticTerm=100.0),
    ]
    return [composite(f, [segment(f, parent, 50.0)]) for parent in parents]


def alignment_curves(f):
    """Horizontal line, a parabolic gradient over it, a constant cant over that."""
    line = f.create_entity("IfcLine", Pnt=pt(f, (0.0, 0.0)),
                           Dir=f.create_entity("IfcVector", Orientation=dr(f, (1.0, 0.0)),
                                               Magnitude=1.0))
    horizontal = composite(f, [segment(f, line, 40.0)])

    # Height over distance: z = 10 + 0.02 d + 0.0005 d^2.
    parabola = f.create_entity(
        "IfcPolynomialCurve", Position=place2(f),
        CoefficientsX=[0.0, 1.0], CoefficientsY=[10.0, 0.02, 0.0005])
    gradient = f.create_entity(
        "IfcGradientCurve",
        Segments=[segment(f, parabola, 40.0)],
        SelfIntersect=False, BaseCurve=horizontal)

    cant_line = f.create_entity("IfcLine", Pnt=pt(f, (0.0, 0.0)),
                                Dir=f.create_entity("IfcVector",
                                                    Orientation=dr(f, (1.0, 0.0)),
                                                    Magnitude=1.0))
    cant = f.create_entity(
        "IfcCurveSegment",
        Transition="DISCONTINUOUS",
        Placement=place3(f),
        SegmentStart=length(f, 0.0),
        SegmentLength=length(f, 40.0),
        ParentCurve=cant_line)
    reference = f.create_entity(
        "IfcSegmentedReferenceCurve",
        Segments=[cant], SelfIntersect=False, BaseCurve=gradient)
    return horizontal, gradient, reference


def linear_position(f, curve, distance):
    """A section position at `distance` along `curve`, with no offsets."""
    location = f.create_entity("IfcPointByDistanceExpression",
                               DistanceAlong=length(f, distance), BasisCurve=curve)
    return f.create_entity("IfcAxis2PlacementLinear", Location=location)


def rectangle(f, name):
    return f.create_entity("IfcRectangleProfileDef", ProfileType="AREA",
                           ProfileName=name, Position=place2(f),
                           XDim=3.0, YDim=0.5)


def open_cross(f, name, slope):
    return f.create_entity("IfcOpenCrossProfileDef", ProfileType="CURVE",
                           ProfileName=name, HorizontalWidths=True,
                           Widths=[3.5, 3.5], Slopes=[-slope, slope],
                           Tags=["left", "axis", "right"])


def swept_items(f, directrix):
    derived = f.create_entity(
        "IfcDirectrixDerivedReferenceSweptAreaSolid",
        SweptArea=rectangle(f, "slab"), Position=place3(f),
        Directrix=directrix,
        StartParam=length(f, 0.0), EndParam=length(f, 40.0),
        FixedReference=dr(f, (0.0, 0.0, 1.0)))
    solid = f.create_entity(
        "IfcSectionedSolidHorizontal",
        Directrix=directrix,
        CrossSections=[rectangle(f, "start"), rectangle(f, "end")],
        CrossSectionPositions=[linear_position(f, directrix, 0.0),
                               linear_position(f, directrix, 40.0)])
    surface = f.create_entity(
        "IfcSectionedSurface",
        Directrix=directrix,
        CrossSectionPositions=[linear_position(f, directrix, 0.0),
                               linear_position(f, directrix, 40.0)],
        CrossSections=[open_cross(f, "start", 0.025), open_cross(f, "end", 0.03)])
    return derived, solid, surface


def offset_curve(f, basis):
    offsets = [
        f.create_entity("IfcPointByDistanceExpression",
                        DistanceAlong=length(f, d), OffsetLateral=lateral,
                        BasisCurve=basis)
        for d, lateral in ((0.0, 2.0), (40.0, 2.5))
    ]
    return f.create_entity("IfcOffsetCurveByDistances", BasisCurve=basis,
                           OffsetValues=offsets, Tag="kerb")


def terrain(f):
    """Two triangles of open terrain; triangle 1 has a breakline on edge 1."""
    points = f.create_entity("IfcCartesianPointList3D", CoordList=[
        (0.0, 0.0, 10.0), (20.0, 0.0, 10.5), (20.0, 20.0, 11.0), (0.0, 20.0, 10.25)])
    return f.create_entity("IfcTriangulatedIrregularNetwork",
                           Coordinates=points, Closed=False,
                           CoordIndex=[(1, 2, 3), (1, 3, 4)], Flags=[1, 0])


def build():
    f = ifcopenshell.file(schema=SCHEMA)
    metre = f.create_entity("IfcSIUnit", UnitType="LENGTHUNIT", Name="METRE")
    radian = f.create_entity("IfcSIUnit", UnitType="PLANEANGLEUNIT", Name="RADIAN")
    units = f.create_entity("IfcUnitAssignment", Units=[metre, radian])
    ctx = f.create_entity("IfcGeometricRepresentationContext", ContextType="Model",
                          CoordinateSpaceDimension=3, Precision=1e-5,
                          WorldCoordinateSystem=place3(f))

    curves = spirals(f)
    horizontal, gradient, reference = alignment_curves(f)
    derived, solid, surface = swept_items(f, gradient)
    offset = offset_curve(f, gradient)
    tin = terrain(f)

    def rep(identifier, kind, items):
        return f.create_entity("IfcShapeRepresentation", ContextOfItems=ctx,
                               RepresentationIdentifier=identifier,
                               RepresentationType=kind, Items=items)

    shape = f.create_entity("IfcProductDefinitionShape", Representations=[
        rep("Axis", "Curve2D", curves + [horizontal]),
        rep("Axis", "Curve3D", [gradient, reference, offset]),
        rep("Body", "AdvancedSweptSolid", [derived, solid]),
        rep("Surface", "SectionedSurface", [surface]),
        rep("Body", "Tessellation", [tin]),
    ])

    def local():
        return f.create_entity("IfcLocalPlacement", RelativePlacement=place3(f))

    project = f.create_entity("IfcProject", GlobalId=guid("project"),
                              Name="IFC4X3 geometry families",
                              UnitsInContext=units, RepresentationContexts=[ctx])
    site = f.create_entity("IfcSite", GlobalId=guid("site"), Name="Site",
                           ObjectPlacement=local())
    carrier = f.create_entity("IfcBuildingElementProxy", GlobalId=guid("carrier"),
                              Name="IFC4X3 geometry carrier",
                              ObjectPlacement=local(), Representation=shape)
    f.create_entity("IfcRelAggregates", GlobalId=guid("aggregates"),
                    RelatingObject=project, RelatedObjects=[site])
    f.create_entity("IfcRelContainedInSpatialStructure", GlobalId=guid("contained"),
                    RelatingStructure=site, RelatedElements=[carrier])
    return f


def main():
    if len(sys.argv) != 2:
        print("usage: gen_ifc4x3_geometry_fixtures.py <outdir>", file=sys.stderr)
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
