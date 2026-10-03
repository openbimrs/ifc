#!/usr/bin/env python3
"""Generate the IFC4X3 ADD2 alignment-curve fixture with ifcopenshell.

`crates/ifc-geometry` lowers `IfcCurveSegment` and `IfcGradientCurve` exactly
(#243) and declares the standalone spirals, `IfcPolynomialCurve` and
`IfcSegmentedReferenceCurve` as typed refusals. The corpus gate
(`tests/lower_dispatch_corpus.rs`) needs a committed instance of each, and
`tests/lower_ifc4x3_curves.rs` checks this file's stations by hand. No
licence-clean public corpus carries this geometry, so it is generated; the
output is our own data.

The geometry is written the way IfcOpenShell's alignment API maps business
logic to geometry (`ifcopenshell.api.alignment`, `_map_alignment_*`):
every `IfcCurveSegment` starts its parent at the inflection point, measures
are `IfcLengthMeasure` arc lengths, and every layout ends in the zero-length
segment IFC4.3 requires.

- Plan (`FootPrint`, `Curve2D`): 100 m line, a clothoid from straight to
  R = 200 m over 60 m (A = sqrt(60 * 200)), a 50 m arc of R = 200 m, the
  closing segment. Plan length 210 m; the arc starts at heading 0.15 rad.
- Profile (`Axis`, `Curve3D`, an `IfcGradientCurve` over the plan): grade
  0.02 over 0..80 from 10 m, a parabola to -0.01 over 80..160
  (`z = 11.6 + 0.02 t - 0.0001875 t^2`), grade -0.01 over 160..210, the
  closing segment at (210, 11.5).
- An `IfcSegmentedReferenceCurve` over the gradient curve, one level cant
  segment and its closing segment (refused: no neutral roll law, #93).
- A transition gallery: one composite per spiral family, each a 0 -> 1/200
  transition over 60 m with IfcOpenShell's term mapping, so every
  `IfcSpiral` subtype has a committed instance.

The clothoid end point is a Fresnel integral; it is summed here from its
power series (terms below 1e-18 m), which is authoring data, exactly as an
exporter computes it. Metres and radians. Entity ids, GUIDs and the header
timestamp are fixed, so regeneration is byte-stable.

Run:  python3 tools/gen_ifc4x3_curve_fixtures.py test/fixtures/synthetic-surfaces
"""

import math
import pathlib
import sys
import uuid

import ifcopenshell
import ifcopenshell.guid

SCHEMA = "IFC4X3_ADD2"
NAME = "synthetic_ifc4x3_alignment_curves.ifc"

RADIUS = 200.0
SPIRAL = 60.0


def guid(label):
    """A GUID that is stable across regenerations."""
    return ifcopenshell.guid.compress(
        uuid.uuid5(uuid.NAMESPACE_URL, "openbimrs/ifc#243/curves/" + label).hex)


def pt(f, xy):
    return f.create_entity("IfcCartesianPoint", Coordinates=[float(v) for v in xy])


def place2(f, origin=(0.0, 0.0), heading=0.0):
    return f.create_entity(
        "IfcAxis2Placement2D", Location=pt(f, origin),
        RefDirection=f.create_entity(
            "IfcDirection", DirectionRatios=[math.cos(heading), math.sin(heading)]))


def place2d(f, origin, direction):
    return f.create_entity(
        "IfcAxis2Placement2D", Location=pt(f, origin),
        RefDirection=f.create_entity("IfcDirection", DirectionRatios=list(direction)))


def length(f, value):
    return f.create_entity("IfcLengthMeasure", float(value))


def segment(f, placement, run, parent, transition="CONTSAMEGRADIENT"):
    return f.create_entity(
        "IfcCurveSegment", Transition=transition, Placement=placement,
        SegmentStart=length(f, 0.0), SegmentLength=length(f, run),
        ParentCurve=parent)


def line(f):
    return f.create_entity(
        "IfcLine", Pnt=pt(f, (0.0, 0.0)),
        Dir=f.create_entity("IfcVector",
                            Orientation=f.create_entity("IfcDirection",
                                                        DirectionRatios=[1.0, 0.0]),
                            Magnitude=1.0))


def composite(f, segments):
    return f.create_entity("IfcCompositeCurve", Segments=segments, SelfIntersect=False)


def clothoid_end(a, s):
    """(x, y) of a clothoid with constant `a` at arc length `s`, by series."""
    q = 2.0 * a * a
    x = y = 0.0
    for n in range(12):
        x += (-1) ** n * s ** (4 * n + 1) / (math.factorial(2 * n) * q ** (2 * n) * (4 * n + 1))
        y += (-1) ** n * s ** (4 * n + 3) / (
            math.factorial(2 * n + 1) * q ** (2 * n + 1) * (4 * n + 3))
    return x, y


def plan(f):
    """Line, clothoid, arc, closing: the horizontal composite."""
    a = math.sqrt(SPIRAL * RADIUS)
    straight = line(f)
    h1 = segment(f, place2(f), 100.0, straight)
    spiral = f.create_entity("IfcClothoid", Position=place2(f), ClothoidConstant=a)
    h2 = segment(f, place2(f, (100.0, 0.0)), SPIRAL, spiral)

    sx, sy = clothoid_end(a, SPIRAL)
    heading = SPIRAL / (2.0 * RADIUS)  # 0.15 rad
    start = (100.0 + sx, sy)
    circle = f.create_entity("IfcCircle", Position=place2(f), Radius=RADIUS)
    h3 = segment(f, place2(f, start, heading), 50.0, circle)

    sweep = 50.0 / RADIUS
    end = (start[0] + RADIUS * (math.sin(heading + sweep) - math.sin(heading)),
           start[1] - RADIUS * (math.cos(heading + sweep) - math.cos(heading)))
    h4 = segment(f, place2(f, end, heading + sweep), 0.0, straight, "DISCONTINUOUS")
    return composite(f, [h1, h2, h3, h4])


def parabola_arc(g, c, x):
    """Arc length of z = g t + c t^2 over [0, x], in closed form."""
    primitive = lambda w: (w * math.sqrt(1.0 + w * w) + math.asinh(w)) / 2.0
    return (primitive(g + 2.0 * c * x) - primitive(g)) / (2.0 * c)


def profile(f):
    """Grade, parabola, grade, closing, in (distance along, height)."""
    straight = line(f)
    tangent = lambda g: (1.0 / math.sqrt(1.0 + g * g), g / math.sqrt(1.0 + g * g))
    v1 = segment(f, place2d(f, (0.0, 10.0), tangent(0.02)),
                 80.0 * math.sqrt(1.0 + 0.02 ** 2), straight)
    c = (-0.01 - 0.02) / (2.0 * 80.0)
    parent = f.create_entity(
        "IfcPolynomialCurve", Position=place2(f),
        CoefficientsX=[0.0, 1.0], CoefficientsY=[11.6, 0.02, c])
    v2 = segment(f, place2d(f, (80.0, 11.6), tangent(0.02)),
                 parabola_arc(0.02, c, 80.0), parent)
    v3 = segment(f, place2d(f, (160.0, 12.0), tangent(-0.01)),
                 50.0 * math.sqrt(1.0 + 0.01 ** 2), straight)
    v4 = segment(f, place2d(f, (210.0, 11.5), tangent(-0.01)), 0.0, straight,
                 "DISCONTINUOUS")
    return [v1, v2, v3, v4]


def cant(f, gradient):
    """One level cant segment over the whole plan, plus its closing segment."""
    def place3(x):
        return f.create_entity(
            "IfcAxis2Placement3D", Location=f.create_entity(
                "IfcCartesianPoint", Coordinates=[x, 0.0, 0.0]))
    level = line(f)
    c1 = segment(f, place3(0.0), 210.0, level)
    c2 = segment(f, place3(210.0), 0.0, level, "DISCONTINUOUS")
    return f.create_entity("IfcSegmentedReferenceCurve", Segments=[c1, c2],
                           SelfIntersect=False, BaseCurve=gradient)


def gallery(f):
    """One 0 -> 1/200 transition over 60 m per spiral family."""
    big_l, k = SPIRAL, SPIRAL / RADIUS  # f = L/R in IfcOpenShell's mapping

    def term(a, power):
        return big_l * abs(a) ** (-1.0 / (power + 1)) * math.copysign(1.0, a)

    def viennese(power, coefficient):
        return math.copysign(abs(coefficient) ** (-1.0 / (power + 1)), coefficient)

    d = 1.0 / RADIUS
    p = lambda: place2(f)
    parents = [
        f.create_entity("IfcClothoid", Position=p(),
                        ClothoidConstant=math.sqrt(SPIRAL * RADIUS)),
        f.create_entity("IfcCosineSpiral", Position=p(),
                        CosineTerm=big_l / (-0.5 * k), ConstantTerm=big_l / (0.5 * k)),
        f.create_entity("IfcSineSpiral", Position=p(),
                        SineTerm=big_l / (-k / (2.0 * math.pi)),
                        LinearTerm=term(k, 1)),
        f.create_entity("IfcSecondOrderPolynomialSpiral", Position=p(),
                        QuadraticTerm=term(k, 2)),
        f.create_entity("IfcThirdOrderPolynomialSpiral", Position=p(),
                        CubicTerm=term(-2.0 * k, 3), QuadraticTerm=term(3.0 * k, 2)),
        # The Viennese bend without cant: 35, -84, 70, -20 (d / L^n) s^n.
        f.create_entity("IfcSeventhOrderPolynomialSpiral", Position=p(),
                        SepticTerm=viennese(7, -20.0 * d / big_l ** 7),
                        SexticTerm=viennese(6, 70.0 * d / big_l ** 6),
                        QuinticTerm=viennese(5, -84.0 * d / big_l ** 5),
                        QuarticTerm=viennese(4, 35.0 * d / big_l ** 4)),
    ]
    curves = []
    for parent in parents:
        # One segment, so it is also the one DISCONTINUOUS segment the
        # schema's CurveContinuous rule asks of an open composite.
        curves.append(composite(f, [segment(f, place2(f), SPIRAL, parent,
                                            "DISCONTINUOUS")]))
    return curves


def build():
    f = ifcopenshell.file(schema=SCHEMA)
    metre = f.create_entity("IfcSIUnit", UnitType="LENGTHUNIT", Name="METRE")
    radian = f.create_entity("IfcSIUnit", UnitType="PLANEANGLEUNIT", Name="RADIAN")
    units = f.create_entity("IfcUnitAssignment", Units=[metre, radian])
    ctx = f.create_entity(
        "IfcGeometricRepresentationContext", ContextType="Model",
        CoordinateSpaceDimension=3, Precision=1e-6,
        WorldCoordinateSystem=f.create_entity(
            "IfcAxis2Placement3D",
            Location=f.create_entity("IfcCartesianPoint", Coordinates=[0.0, 0.0, 0.0])))

    horizontal = plan(f)
    gradient = f.create_entity("IfcGradientCurve", Segments=profile(f),
                               SelfIntersect=False, BaseCurve=horizontal)
    reference = cant(f, gradient)
    transitions = gallery(f)

    def rep(identifier, kind, items):
        return f.create_entity("IfcShapeRepresentation", ContextOfItems=ctx,
                               RepresentationIdentifier=identifier,
                               RepresentationType=kind, Items=items)

    def local():
        return f.create_entity(
            "IfcLocalPlacement", RelativePlacement=f.create_entity(
                "IfcAxis2Placement3D", Location=f.create_entity(
                    "IfcCartesianPoint", Coordinates=[0.0, 0.0, 0.0])))

    project = f.create_entity("IfcProject", GlobalId=guid("project"),
                              Name="IFC4X3 alignment curves",
                              UnitsInContext=units, RepresentationContexts=[ctx])
    site = f.create_entity("IfcSite", GlobalId=guid("site"), Name="Site",
                           ObjectPlacement=local())
    centreline = f.create_entity(
        "IfcBuildingElementProxy", GlobalId=guid("centreline"),
        Name="Alignment centreline carrier", ObjectPlacement=local(),
        Representation=f.create_entity("IfcProductDefinitionShape", Representations=[
            rep("FootPrint", "Curve2D", [horizontal]),
            rep("Axis", "Curve3D", [gradient, reference]),
        ]))
    gallery_carrier = f.create_entity(
        "IfcBuildingElementProxy", GlobalId=guid("gallery"),
        Name="Transition spiral gallery", ObjectPlacement=local(),
        Representation=f.create_entity("IfcProductDefinitionShape", Representations=[
            rep("Axis", "Curve2D", transitions),
        ]))
    f.create_entity("IfcRelAggregates", GlobalId=guid("aggregates"),
                    RelatingObject=project, RelatedObjects=[site])
    f.create_entity("IfcRelContainedInSpatialStructure", GlobalId=guid("contained"),
                    RelatingStructure=site, RelatedElements=[centreline, gallery_carrier])
    return f


def main():
    if len(sys.argv) != 2:
        print("usage: gen_ifc4x3_curve_fixtures.py <outdir>", file=sys.stderr)
        return 2
    out = pathlib.Path(sys.argv[1])
    out.mkdir(parents=True, exist_ok=True)
    model = build()
    model.header.file_name.time_stamp = "1970-01-01T00:00:00"
    target = out / NAME
    model.write(str(target))
    print(f"{target.stat().st_size:>7} {NAME}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
