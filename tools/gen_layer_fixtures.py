#!/usr/bin/env python3
"""Generate the material layer set usage fixtures for #455.

Each file is a project with a 3D `Model` context, a `Body` and an `Axis`
sub-context, and walls and slabs whose material is an
`IfcMaterialLayerSetUsage` (or, as negative controls, a bare layer set and a
plain material). They state layered products whose layer boundary planes are
known by hand. No licence-clean public corpus carries these, so they are
generated; the output is our own data. Fixed GUIDs and time stamp make
regeneration byte-stable, and each file is checked with
`ifcopenshell.validate` (EXPRESS rules included) before it is written.
Generated with ifcopenshell 0.9.0.post1.

Layer sets (thicknesses in millimetres):

| Set            | Layers                                                        |
| -------------- | ------------------------------------------------------------- |
| Cavity Brick   | Brick 115, MineralWool 100, air (no material) 40, Brick 115   |
| Single Concrete| Concrete 200                                                  |
| Floor          | Screed 50, Insulation 30, Concrete 200                        |
| Offsets        | one `IfcMaterialLayerWithOffsets`, Concrete 200, offsets      |
|                | along AXIS3 of (0, 500) (IFC4 and IFC4X3 only)                |

Products (metres, object frame; every wall is 5 m long along its local x axis,
2.8 m high, with a rectangle profile centred on its layers):

| Product          | Placement, usage                                  | Boundaries, layer order            |
| ---------------- | ------------------------------------------------- | ---------------------------------- |
| `CavityWall`     | (1000, 2000, 0) turned 90 deg; Axis (0,0)-(5000,0);| -.185 -.07 .03 .07 .185; 0 1 2 3  |
|                  | AXIS2 POSITIVE offset -185                         | (`IfcWallStandardCase` but IFC4X3) |
| `ReversedWall`   | as `CavityWall`; NEGATIVE offset +185              | -.185 -.07 -.03 .07 .185; 3 2 1 0  |
| `ClippedWall`    | as `CavityWall`, body clipped by a half space at   | as `CavityWall`                    |
|                  | z 2400 (IFC4 and IFC4X3)                           |                                    |
| `OffsetAxisWall` | Axis at y = 100; Single Concrete, POSITIVE 0       | .1 .3                              |
| `FloorSlab`      | at z 3000; extrusion Position z -280, depth 280;   | -.28 -.08 -.05 0; 2 1 0; normal z  |
|                  | Floor, AXIS3 NEGATIVE offset +280                  | (`IfcSlabStandardCase` in IFC4)    |
| `RoofSlab`       | Position Axis (0,-.5,.866), RefDirection (1,0,0),  | 0 .2; normal (0,-.5,.866)          |
|                  | ExtrudedDirection (0,.5,.866), Depth 200/cos30;    | (IFC4 and IFC4X3)                  |
|                  | Single Concrete,                                   |                                    |
|                  | AXIS3 POSITIVE 0                                   |                                    |
| `CurvedWall`     | Axis is an `IfcTrimmedCurve` on an `IfcCircle`     | refused: curve unsupported         |
| `NoAxisWall`     | no Axis representation                             | refused: missing Axis              |
| `OffsetLayersWall`| the Offsets set                                   | refused: layer with offsets        |
| `PlainSetWall`   | a bare `IfcMaterialLayerSet`                       | no usage                           |
| `Column`         | an `IfcMaterial`                                   | no usage                           |

`material_layers.ifc` is IFC4 in millimetres, `material_layers_ifc4x3.ifc`
IFC4X3_ADD2 in metres (`IfcWall` and `IfcSlab` only), and
`material_layers_ifc2x3.ifc` IFC2X3 in millimetres with an owner history and
only `CavityWall`, `ReversedWall`, `FloorSlab`, `PlainSetWall` and `Column`.

Run:  python3 tools/gen_layer_fixtures.py test/fixtures/synthetic-layers
"""

import logging
import math
import pathlib
import sys
import uuid

import ifcopenshell
import ifcopenshell.guid
import ifcopenshell.validate

ROOF = math.sqrt(3.0) / 2.0  # 0.8660254037844386


class Builder:
    """One file. `length` scales millimetres to the file's length unit."""

    def __init__(self, schema, label, length):
        self.f = ifcopenshell.file(schema=schema)
        self.schema = schema
        self.label = label
        self.ifc2x3 = schema == "IFC2X3"
        self.ifc4x3 = schema == "IFC4X3_ADD2"
        self.length = length
        self.materials = {}
        self.sets = {}
        self.owner = self._owner_history() if self.ifc2x3 else None
        self.spatial()

    def guid(self, name):
        """A GUID derived from a label, so regeneration is byte-stable."""
        return ifcopenshell.guid.compress(
            uuid.uuid5(uuid.NAMESPACE_URL,
                       f"openbimrs/ifc#455/{self.label}/{name}").hex)

    def rooted(self, name, **attributes):
        """The attributes every rooted entity carries."""
        owner = {"OwnerHistory": self.owner} if self.ifc2x3 else {}
        return {"GlobalId": self.guid(name), "Name": name, **owner, **attributes}

    def _owner_history(self):
        f = self.f
        person = f.create_entity("IfcPerson", FamilyName="Synthetic")
        organization = f.create_entity("IfcOrganization", Name="openbimrs")
        user = f.create_entity("IfcPersonAndOrganization", ThePerson=person,
                               TheOrganization=organization)
        application = f.create_entity(
            "IfcApplication", ApplicationDeveloper=organization, Version="1",
            ApplicationFullName="gen_layer_fixtures.py",
            ApplicationIdentifier="gen_layer_fixtures")
        return f.create_entity("IfcOwnerHistory", OwningUser=user,
                               OwningApplication=application,
                               ChangeAction="NOCHANGE", CreationDate=0)

    def spatial(self):
        f = self.f
        length = f.create_entity("IfcSIUnit", UnitType="LENGTHUNIT", Name="METRE")
        if self.length == 1.0:
            length.Prefix = "MILLI"
        angle = f.create_entity("IfcSIUnit", UnitType="PLANEANGLEUNIT", Name="RADIAN")
        units = f.create_entity("IfcUnitAssignment", Units=[length, angle])
        self.model = f.create_entity(
            "IfcGeometricRepresentationContext", ContextType="Model",
            CoordinateSpaceDimension=3, Precision=1.0e-5,
            WorldCoordinateSystem=self.placement3d((0, 0, 0)))
        self.body = f.create_entity(
            "IfcGeometricRepresentationSubContext", ContextIdentifier="Body",
            ContextType="Model", ParentContext=self.model, TargetView="MODEL_VIEW")
        self.axis_context = f.create_entity(
            "IfcGeometricRepresentationSubContext", ContextIdentifier="Axis",
            ContextType="Model", ParentContext=self.model, TargetView="GRAPH_VIEW")
        attributes = {"OwnerHistory": self.owner} if self.ifc2x3 else {}
        self.project = f.create_entity(
            "IfcProject", GlobalId=self.guid("project"),
            Name=f"#455 material layers ({self.schema})", UnitsInContext=units,
            RepresentationContexts=[self.model], **attributes)

    # Measures in the file's units.
    def len(self, millimetres):
        return round(millimetres * self.length, 9)

    def point(self, *coordinates):
        return self.f.create_entity("IfcCartesianPoint",
                                    Coordinates=[float(c) for c in coordinates])

    def direction(self, *ratios):
        return self.f.create_entity("IfcDirection",
                                    DirectionRatios=[float(r) for r in ratios])

    def placement3d(self, location, axis=None, ref=None):
        """An `IfcAxis2Placement3D`; `location` in millimetres."""
        return self.f.create_entity(
            "IfcAxis2Placement3D",
            Location=self.point(*[self.len(c) for c in location]),
            Axis=self.direction(*axis) if axis else None,
            RefDirection=self.direction(*ref) if ref else None)

    def local_placement(self, location, ref=None):
        return self.f.create_entity("IfcLocalPlacement",
                                    RelativePlacement=self.placement3d(
                location, axis=(0, 0, 1) if ref else None, ref=ref))

    # Materials and layer sets.
    def material(self, name):
        if name not in self.materials:
            self.materials[name] = self.f.create_entity("IfcMaterial", Name=name)
        return self.materials[name]

    def layer(self, material, thickness, offsets=None):
        f = self.f
        attributes = {"Material": self.material(material) if material else None,
                      "LayerThickness": self.len(thickness)}
        if material is None:
            attributes["IsVentilated"] = True
        if offsets:
            return f.create_entity(
                "IfcMaterialLayerWithOffsets", OffsetDirection="AXIS3",
                OffsetValues=[self.len(v) for v in offsets], **attributes)
        return f.create_entity("IfcMaterialLayer", **attributes)

    def layer_set(self, name):
        """The shared `IfcMaterialLayerSet` named `name`."""
        if name not in self.sets:
            layers = {
                "Cavity Brick": [("Brick", 115), ("MineralWool", 100), (None, 40),
                                 ("Brick", 115)],
                "Single Concrete": [("Concrete", 200)],
                "Floor": [("Screed", 50), ("Insulation", 30), ("Concrete", 200)],
            }
            if name == "Offsets":
                made = [self.layer("Concrete", 200, offsets=(0, 500))]
            else:
                made = [self.layer(m, t) for m, t in layers[name]]
            self.sets[name] = self.f.create_entity(
                "IfcMaterialLayerSet", MaterialLayers=made, LayerSetName=name)
        return self.sets[name]

    def usage(self, set_name, direction, sense, offset):
        attributes = {}
        if not self.ifc2x3:
            attributes["ReferenceExtent"] = self.len(2800)
        return self.f.create_entity(
            "IfcMaterialLayerSetUsage", ForLayerSet=self.layer_set(set_name),
            LayerSetDirection=direction, DirectionSense=sense,
            OffsetFromReferenceLine=self.len(offset), **attributes)

    def associate(self, product, material):
        self.f.create_entity(
            "IfcRelAssociatesMaterial",
            **self.rooted(f"{product.Name} material", RelatedObjects=[product],
                          RelatingMaterial=material))

    # Bodies.
    def rectangle(self, x, y, centre):
        return self.f.create_entity(
            "IfcRectangleProfileDef", ProfileType="AREA",
            XDim=self.len(x), YDim=self.len(y),
            Position=self.f.create_entity(
                "IfcAxis2Placement2D",
                Location=self.point(*[self.len(c) for c in centre])))

    def extrusion(self, profile, depth, position, direction=(0, 0, 1)):
        return self.f.create_entity(
            "IfcExtrudedAreaSolid", SweptArea=profile, Position=position,
            ExtrudedDirection=self.direction(*direction), Depth=self.len(depth))

    def shape(self, identifier, kind, items, context):
        return self.f.create_entity(
            "IfcShapeRepresentation", ContextOfItems=context,
            RepresentationIdentifier=identifier, RepresentationType=kind, Items=items)

    def definition(self, *representations):
        return self.f.create_entity("IfcProductDefinitionShape",
                                    Representations=list(representations))

    def polyline(self, y):
        return self.f.create_entity(
            "IfcPolyline", Points=[self.point(0, self.len(y)),
                                   self.point(self.len(5000), self.len(y))])

    def curved_axis(self):
        f = self.f
        circle = f.create_entity(
            "IfcCircle", Radius=self.len(4000),
            Position=f.create_entity("IfcAxis2Placement2D",
                                     Location=self.point(0, 0)))
        return f.create_entity(
            "IfcTrimmedCurve", BasisCurve=circle,
            Trim1=[f.create_entity("IfcParameterValue", 0.0)],
            Trim2=[f.create_entity("IfcParameterValue", 1.0)],
            SenseAgreement=True, MasterRepresentation="PARAMETER")

    def clipped(self, solid):
        f = self.f
        plane = f.create_entity("IfcPlane", Position=self.placement3d((0, 0, 2400)))
        half = f.create_entity("IfcHalfSpaceSolid", BaseSurface=plane,
                               AgreementFlag=False)
        return f.create_entity("IfcBooleanClippingResult", Operator="DIFFERENCE",
                               FirstOperand=solid, SecondOperand=half)

    # Products.
    def wall(self, name, entity, thickness, centre, usage=None, axis="straight",
             y=0, clip=False, material=None, placement=(1000, 2000, 0)):
        profile = self.rectangle(5000, thickness, (2500, centre))
        solid = self.extrusion(profile, 2800, self.placement3d((0, 0, 0)))
        kind = "SweptSolid"
        if clip:
            solid, kind = self.clipped(solid), "Clipping"
        representations = [self.shape("Body", kind, [solid], self.body)]
        if axis == "straight":
            representations.append(
                self.shape("Axis", "Curve2D", [self.polyline(y)], self.axis_context))
        elif axis == "curved":
            representations.append(
                self.shape("Axis", "Curve2D", [self.curved_axis()], self.axis_context))
        wall = self.f.create_entity(
            entity, **self.rooted(
                name, ObjectPlacement=self.local_placement(placement, ref=(0, 1, 0)),
                Representation=self.definition(*representations)))
        self.associate(wall, usage or material)
        return wall

    def slab(self, name, entity, position, direction, profile, depth, usage,
             predefined, z):
        solid = self.extrusion(profile, depth, position, direction)
        slab = self.f.create_entity(
            entity, **self.rooted(
                name, ObjectPlacement=self.local_placement((0, 0, z)),
                Representation=self.definition(
                    self.shape("Body", "SweptSolid", [solid], self.body)),
                PredefinedType=predefined))
        self.associate(slab, usage)
        return slab


def build(schema, label, length):
    b = Builder(schema, label, length)
    ifc4x3 = b.ifc4x3
    standard_wall = "IfcWall" if ifc4x3 else "IfcWallStandardCase"
    standard_slab = {"IFC4": "IfcSlabStandardCase"}.get(schema, "IfcSlab")

    cavity = lambda: b.usage("Cavity Brick", "AXIS2", "POSITIVE", -185)
    b.wall("CavityWall", standard_wall, 370, 0, cavity())
    b.wall("ReversedWall", "IfcWall", 370, 0,
           b.usage("Cavity Brick", "AXIS2", "NEGATIVE", 185))
    if not b.ifc2x3:
        b.wall("ClippedWall", "IfcWall", 370, 0, cavity(), clip=True)
        b.wall("OffsetAxisWall", "IfcWall", 200, 200,
               b.usage("Single Concrete", "AXIS2", "POSITIVE", 0), y=100)
    b.slab("FloorSlab", standard_slab, b.placement3d((0, 0, -280)), (0, 0, 1),
           b.rectangle(4000, 3000, (0, 0)), 280,
           b.usage("Floor", "AXIS3", "NEGATIVE", 280), "FLOOR", 3000)
    if not b.ifc2x3:
        tilted = b.placement3d((0, 0, 0), axis=(0, -0.5, ROOF), ref=(1, 0, 0))
        b.slab("RoofSlab", "IfcSlab", tilted, (0, 0.5, ROOF),
               b.rectangle(4000, 3000, (0, 0)), 200 / ROOF,
               b.usage("Single Concrete", "AXIS3", "POSITIVE", 0), "ROOF", 6000)
        b.wall("CurvedWall", "IfcWall", 370, 0, cavity(), axis="curved")
        b.wall("NoAxisWall", "IfcWall", 370, 0, cavity(), axis=None)
        b.wall("OffsetLayersWall", "IfcWall", 200, 0,
               b.usage("Offsets", "AXIS2", "POSITIVE", 0))
    b.wall("PlainSetWall", "IfcWall", 370, 0,
           material=b.layer_set("Cavity Brick"))
    column = b.f.create_entity(
        "IfcColumn", **b.rooted("Column", ObjectPlacement=b.local_placement((0, 0, 0))))
    b.associate(column, b.material("Concrete"))
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


def main():
    if len(sys.argv) != 2:
        print("usage: gen_layer_fixtures.py <outdir>", file=sys.stderr)
        return 2
    out = pathlib.Path(sys.argv[1])
    out.mkdir(parents=True, exist_ok=True)
    description = "openbimrs/ifc #455: synthetic material layer set usage fixture, AGPL-3.0-or-later"
    write(build("IFC4", "IFC4", 1.0), out, "material_layers.ifc", description)
    write(build("IFC4X3_ADD2", "IFC4X3_ADD2", 0.001), out,
          "material_layers_ifc4x3.ifc", description)
    write(build("IFC2X3", "IFC2X3", 1.0), out, "material_layers_ifc2x3.ifc",
          description)
    return 0


if __name__ == "__main__":
    sys.exit(main())
