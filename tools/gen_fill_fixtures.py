#!/usr/bin/env python3
"""Generate the material cut-fill fixtures for #597.

Each file is a project with a 3D `Model` and a 2D `Plan` context and a set of
`IfcMaterial`s. A styled material is the chain
`IfcMaterialDefinitionRepresentation` -> `IfcStyledRepresentation`
(`Style` / `Material`) -> item-less `IfcStyledItem` -> `IfcFillAreaStyle`.
No licence-clean public corpus carries these, so they are generated; the
output is our own data. Fixed GUIDs and time stamp make regeneration
byte-stable, and each file is checked with `ifcopenshell.validate` (EXPRESS
rules included) before it is written. Generated with ifcopenshell
0.9.0.post1.

Materials (metres and radians; `material_fill_mm_deg.ifc` states the same
fills in millimetres and degrees):

| Material       | Fill                                                              |
| -------------- | ----------------------------------------------------------------- |
| `Concrete`     | 'Concrete cut': background (0.9, 0.9, 0.9) and two hatchings. H1: |
|                | 45 deg, 5 mm apart, 'continuous', 0.25 mm wide, colour 0.2 grey.  |
|                | H2: 0 deg, spacing by `IfcVector((0.6, 0.8))` of 5 mm, reference  |
|                | point (0.5 mm, 0), pattern start (1 mm, 0), 2 mm / 1 mm dashes,   |
|                | predefined 'black', no width. Resolves to spacing 4 mm, shift     |
|                | 3 mm.                                                             |
| `Insulation`   | 'Insulation cut': predefined 'white' only, a solid fill.          |
| `Scaled`       | One 90 deg hatching 10 mm apart, the dash font scaled by 2.       |
| `Layered`      | Model context 'Layered model' and Plan context 'Layered plan',    |
|                | which differ in spacing: ambiguous.                               |
| `Shared`       | Model and Plan contexts, each its own styled item, both on        |
|                | 'Insulation cut': one fill.                                       |
| `Tiled`        | `IfcFillAreaStyleTiles` (refused; not in IFC2X3).                 |
| `External`     | `IfcExternallyDefinedHatchStyle` (refused).                       |
| `ByLayer`      | A hatch line coloured 'by layer' (refused).                       |
| `Dashed`       | A hatch line in the predefined font 'dashed' (refused).           |
| `SurfaceOnly`  | A styled item with only an `IfcSurfaceStyle`: no fill.            |
| `Plain`        | No representation: no fill.                                       |

`material_fill.ifc` is IFC4, `material_fill_ifc4x3.ifc` IFC4X3_ADD2 (its
`ModelOrDraughting` flag and `CurveStyleFont` slot), and
`material_fill_ifc2x3.ifc` IFC2X3 TC1: styled items through
`IfcPresentationStyleAssignment`, an owner history on the project, the H2
vector wrapped in an `IfcOneDirectionRepeatFactor`, no `ModelOrDraughting`
flag and no `Tiled`. `material_fill_mm_deg.ifc` is IFC4 in millimetres and
degrees with `Concrete` and `Scaled` only.

Run:  python3 tools/gen_fill_fixtures.py test/fixtures/synthetic-fill
"""

import logging
import pathlib
import sys
import uuid

import ifcopenshell
import ifcopenshell.guid
import ifcopenshell.validate

DEGREE = 0.017453292519943295


class Builder:
    """One file. `length` and `angle` scale metres and radians to the file's units."""

    def __init__(self, schema, label, length=1.0, angle=1.0, units="si"):
        self.f = ifcopenshell.file(schema=schema)
        self.schema = schema
        self.label = label
        self.ifc2x3 = schema == "IFC2X3"
        self.ifc4x3 = schema == "IFC4X3_ADD2"
        self.length = length
        self.angle = angle
        self.fills = {}
        self.owner = self._owner_history() if self.ifc2x3 else None
        self.spatial(units)

    def guid(self, name):
        """A GUID derived from a label, so regeneration is byte-stable."""
        return ifcopenshell.guid.compress(
            uuid.uuid5(uuid.NAMESPACE_URL,
                       f"openbimrs/ifc#597/{self.label}/{name}").hex)

    def _owner_history(self):
        f = self.f
        person = f.create_entity("IfcPerson", FamilyName="Synthetic")
        organization = f.create_entity("IfcOrganization", Name="openbimrs")
        user = f.create_entity("IfcPersonAndOrganization", ThePerson=person,
                               TheOrganization=organization)
        application = f.create_entity(
            "IfcApplication", ApplicationDeveloper=organization, Version="1",
            ApplicationFullName="gen_fill_fixtures.py",
            ApplicationIdentifier="gen_fill_fixtures")
        return f.create_entity("IfcOwnerHistory", OwningUser=user,
                               OwningApplication=application,
                               ChangeAction="NOCHANGE", CreationDate=0)

    def spatial(self, units):
        f = self.f
        if units == "si":
            assigned = [
                f.create_entity("IfcSIUnit", UnitType="LENGTHUNIT", Name="METRE"),
                f.create_entity("IfcSIUnit", UnitType="PLANEANGLEUNIT", Name="RADIAN"),
            ]
        else:
            degree = f.create_entity(
                "IfcConversionBasedUnit",
                Dimensions=f.create_entity("IfcDimensionalExponents",
                                           0, 0, 0, 0, 0, 0, 0),
                UnitType="PLANEANGLEUNIT", Name="DEGREE",
                ConversionFactor=f.create_entity(
                    "IfcMeasureWithUnit",
                    ValueComponent=f.create_entity("IfcPlaneAngleMeasure", DEGREE),
                    UnitComponent=f.create_entity("IfcSIUnit", UnitType="PLANEANGLEUNIT",
                                                  Name="RADIAN")))
            assigned = [
                f.create_entity("IfcSIUnit", UnitType="LENGTHUNIT", Prefix="MILLI",
                                Name="METRE"),
                degree,
            ]
        unit_assignment = f.create_entity("IfcUnitAssignment", Units=assigned)
        self.model = f.create_entity(
            "IfcGeometricRepresentationContext", ContextType="Model",
            CoordinateSpaceDimension=3, Precision=1.0e-5,
            WorldCoordinateSystem=f.create_entity(
                "IfcAxis2Placement3D", Location=self.point(0, 0, 0)))
        self.plan = f.create_entity(
            "IfcGeometricRepresentationContext", ContextType="Plan",
            CoordinateSpaceDimension=2, Precision=1.0e-5,
            WorldCoordinateSystem=f.create_entity(
                "IfcAxis2Placement2D", Location=self.point(0, 0)))
        attributes = {"OwnerHistory": self.owner} if self.ifc2x3 else {}
        self.project = f.create_entity(
            "IfcProject", GlobalId=self.guid("project"),
            Name=f"#597 material fill ({self.schema})", UnitsInContext=unit_assignment,
            RepresentationContexts=[self.model, self.plan], **attributes)

    # Measures in the file's units.
    def len(self, metres):
        return round(metres * self.length, 9)

    def ang(self, radians):
        # Radians pass through exactly; degrees are rounded to the nanodegree.
        return radians if self.angle == 1.0 else round(radians * self.angle, 9)

    def point(self, *coordinates):
        return self.f.create_entity("IfcCartesianPoint",
                                    Coordinates=[float(c) for c in coordinates])

    def colour(self, red, green, blue):
        return self.f.create_entity("IfcColourRgb", Red=red, Green=green, Blue=blue)

    def predefined_colour(self, name):
        return self.f.create_entity("IfcDraughtingPreDefinedColour", Name=name)

    def dash_font(self, name="dash"):
        f = self.f
        pattern = f.create_entity(
            "IfcCurveStyleFontPattern", VisibleSegmentLength=self.len(0.002),
            InvisibleSegmentLength=self.len(0.001))
        return f.create_entity("IfcCurveStyleFont", Name=name, PatternList=[pattern])

    def curve_style(self, font=None, width=None, colour=None):
        return self.f.create_entity("IfcCurveStyle", CurveFont=font, CurveWidth=width,
                                    CurveColour=colour)

    def hatching(self, appearance, distance, angle, reference=None, start=None):
        return self.f.create_entity(
            "IfcFillAreaStyleHatching", HatchLineAppearance=appearance,
            StartOfNextHatchLine=distance, PointOfReferenceHatchLine=reference,
            PatternStart=start, HatchLineAngle=self.ang(angle))

    def length_distance(self, metres):
        return self.f.create_entity("IfcPositiveLengthMeasure", self.len(metres))

    def fill(self, name, styles, flag=True):
        """An `IfcFillAreaStyle`; shared by name across the materials using it."""
        if name in self.fills:
            return self.fills[name]
        attributes = {}
        if self.ifc4x3:
            attributes["ModelOrDraughting"] = flag
        elif not self.ifc2x3:
            attributes["ModelorDraughting"] = flag
        style = self.f.create_entity("IfcFillAreaStyle", Name=name, FillStyles=styles,
                                     **attributes)
        self.fills[name] = style
        return style

    def styled_item(self, style, item=None):
        styles = [style]
        if self.ifc2x3:
            styles = [self.f.create_entity("IfcPresentationStyleAssignment",
                                           Styles=styles)]
        return self.f.create_entity("IfcStyledItem", Item=item, Styles=styles)

    def material(self, name, styled=()):
        """An `IfcMaterial`; `styled` lists (context, style) pairs, one styled
        representation each."""
        f = self.f
        material = f.create_entity("IfcMaterial", Name=name)
        representations = [
            f.create_entity(
                "IfcStyledRepresentation", ContextOfItems=context,
                RepresentationIdentifier="Style", RepresentationType="Material",
                Items=[self.styled_item(style)])
            for context, style in styled
        ]
        if representations:
            f.create_entity("IfcMaterialDefinitionRepresentation",
                            Representations=representations,
                            RepresentedMaterial=material)
        return material

    # The fills of the table in the module docstring.
    def concrete(self):
        f = self.f
        h1 = self.hatching(
            self.curve_style(
                f.create_entity("IfcDraughtingPreDefinedCurveFont", Name="continuous"),
                self.length_distance(0.00025), self.colour(0.2, 0.2, 0.2)),
            self.length_distance(0.005), 0.7853981633974483)
        vector = f.create_entity(
            "IfcVector",
            Orientation=f.create_entity("IfcDirection", DirectionRatios=[0.6, 0.8]),
            Magnitude=self.len(0.005))
        if self.ifc2x3:
            vector = f.create_entity("IfcOneDirectionRepeatFactor", RepeatFactor=vector)
        h2 = self.hatching(
            self.curve_style(self.dash_font(), None, self.predefined_colour("black")),
            vector, 0.0,
            reference=self.point(self.len(0.0005), 0.0),
            start=self.point(self.len(0.001), 0.0))
        return self.fill("Concrete cut", [h1, h2, self.colour(0.9, 0.9, 0.9)])

    def insulation(self):
        return self.fill("Insulation cut", [self.predefined_colour("white")],
                         flag=None)

    def scaled(self):
        slot = "CurveStyleFont" if self.ifc4x3 else "CurveFont"
        font = self.f.create_entity("IfcCurveStyleFontAndScaling", Name="dash x2",
                                    CurveFontScaling=2.0,
                                    **{slot: self.dash_font()})
        h = self.hatching(self.curve_style(font), self.length_distance(0.01),
                          1.5707963267948966)
        return self.fill("Scaled cut", [h])

    def continuous_hatch(self, metres):
        font = self.f.create_entity("IfcDraughtingPreDefinedCurveFont",
                                    Name="continuous")
        return self.hatching(self.curve_style(font), self.length_distance(metres), 0.0)

    def tiled(self):
        f = self.f
        polyline = f.create_entity(
            "IfcPolyline", Points=[self.point(0, 0), self.point(self.len(0.01), 0)])
        tile = self.styled_item(self.curve_style(
            f.create_entity("IfcDraughtingPreDefinedCurveFont", Name="continuous")),
            polyline)
        vectors = [
            f.create_entity(
                "IfcVector",
                Orientation=f.create_entity("IfcDirection", DirectionRatios=axis),
                Magnitude=self.len(0.01))
            for axis in ([1.0, 0.0], [0.0, 1.0])
        ]
        tiles = f.create_entity("IfcFillAreaStyleTiles", TilingPattern=vectors,
                                Tiles=[tile], TilingScale=1.0)
        return self.fill("Tiled cut", [tiles])

    def external(self):
        reference = "ItemReference" if self.ifc2x3 else "Identification"
        hatch = self.f.create_entity("IfcExternallyDefinedHatchStyle",
                                     Location="acad.pat", Name="ANSI31",
                                     **{reference: "ANSI31"})
        return self.fill("External cut", [hatch])

    def by_layer(self):
        font = self.f.create_entity("IfcDraughtingPreDefinedCurveFont",
                                    Name="continuous")
        h = self.hatching(
            self.curve_style(font, None, self.predefined_colour("by layer")),
            self.length_distance(0.005), 0.0)
        return self.fill("By layer cut", [h])

    def dashed(self):
        font = self.f.create_entity("IfcDraughtingPreDefinedCurveFont", Name="dashed")
        h = self.hatching(self.curve_style(font), self.length_distance(0.005), 0.0)
        return self.fill("Dashed cut", [h])

    def surface_only(self):
        f = self.f
        shading = f.create_entity("IfcSurfaceStyleShading",
                                  SurfaceColour=self.colour(0.5, 0.5, 0.5))
        style = f.create_entity("IfcSurfaceStyle", Name="Grey", Side="BOTH",
                                Styles=[shading])
        return style


def build(schema, label, minimal=False, **units):
    b = Builder(schema, label, **units)
    b.material("Concrete", [(b.plan, b.concrete())])
    b.material("Scaled", [(b.plan, b.scaled())])
    if minimal:
        return b.f
    b.material("Insulation", [(b.plan, b.insulation())])
    layered_model = b.fill("Layered model", [b.continuous_hatch(0.005)])
    layered_plan = b.fill("Layered plan", [b.continuous_hatch(0.01)])
    b.material("Layered", [(b.model, layered_model), (b.plan, layered_plan)])
    b.material("Shared", [(b.model, b.insulation()), (b.plan, b.insulation())])
    if not b.ifc2x3:
        b.material("Tiled", [(b.plan, b.tiled())])
    b.material("External", [(b.plan, b.external())])
    b.material("ByLayer", [(b.plan, b.by_layer())])
    b.material("Dashed", [(b.plan, b.dashed())])
    b.material("SurfaceOnly", [(b.plan, b.surface_only())])
    b.material("Plain")
    return b.f


def write(model, out, name, description):
    model.header.file_name.name = name
    model.header.file_description.description = (description,)
    # A fixed timestamp keeps regeneration byte-stable.
    model.header.file_name.time_stamp = "1970-01-01T00:00:00"

    logger = ifcopenshell.validate.json_logger()
    ifcopenshell.validate.validate(model, logger, express_rules=True)
    if logger.statements:
        for statement in logger.statements:
            print(statement, file=sys.stderr)
        raise SystemExit(f"{name} does not validate")
    logging.getLogger().handlers.clear()

    target = out / name
    model.write(str(target))
    print(f"{target.stat().st_size:>7} {name}")
    for material in model.by_type("IfcMaterial"):
        print(f"        #{material.id()} {material.Name}")


def main():
    if len(sys.argv) != 2:
        print("usage: gen_fill_fixtures.py <outdir>", file=sys.stderr)
        return 2
    out = pathlib.Path(sys.argv[1])
    out.mkdir(parents=True, exist_ok=True)
    description = "openbimrs/ifc #597: synthetic material cut-fill fixture, AGPL-3.0-or-later"
    write(build("IFC4", "IFC4"), out, "material_fill.ifc", description)
    write(build("IFC4X3_ADD2", "IFC4X3_ADD2"), out, "material_fill_ifc4x3.ifc",
          description)
    write(build("IFC2X3", "IFC2X3"), out, "material_fill_ifc2x3.ifc", description)
    write(build("IFC4", "IFC4-mm-deg", minimal=True, length=1000.0,
                angle=1.0 / DEGREE, units="mm_deg"),
          out, "material_fill_mm_deg.ifc", description)
    return 0


if __name__ == "__main__":
    sys.exit(main())
