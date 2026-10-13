#!/usr/bin/env python3
"""Generate the surface-colour fixtures for #449.

Three small files, one per release family, each a row of 1 m cubes whose
colour comes from a different place: a styled item on the representation
item, a presentation layer with style, or the material's
`IfcMaterialDefinitionRepresentation` -> `IfcStyledRepresentation` ->
`IfcStyledItem` -> `IfcSurfaceStyle`. No licence-clean public corpus
isolates these cases, so they are generated; the output is our own data.
Metres and radians, fixed GUIDs and time stamp: regeneration is
byte-stable, and each file is checked with `ifcopenshell.validate`
(EXPRESS rules included) before it is written.

Colours (`IfcSurfaceStyleRendering`, so IFC2X3 can carry transparency):

| Material / style | RGB              | Transparency |
| ---------------- | ---------------- | ------------ |
| `Red`            | (0.8, 0.1, 0.1)  | 0.25         |
| `Green`          | (0.1, 0.7, 0.2)  | 0            |
| `Glass`          | (0.6, 0.8, 0.9)  | 0.6          |
| item `Blue`      | (0.1, 0.2, 0.9)  | 0            |
| layer `Yellow`   | (0.9, 0.8, 0.1)  | 0            |
| `Plain`          | no representation                |

`material_colour.ifc` (IFC4) and `material_colour_ifc4x3.ifc`
(IFC4X3_ADD2) state the same products, in this order:

- `MATERIAL`: an `IfcMaterial` (`Red`) only.
- `LAYER_SET`: an `IfcWall` with an `IfcMaterialLayerSetUsage` of a
  one-layer set (`Green`).
- `ITEM_OVER_MATERIAL`: material `Red`, and an `IfcStyledItem` (`Blue`) on
  the extrusion: the item wins.
- `LAYER_OVER_MATERIAL`: material `Red`, and the extrusion on an
  `IfcPresentationLayerWithStyle` (`Yellow`): the layer wins.
- `MULTI_LAYER`: an `IfcWall` with a two-layer set (`Red`, `Green`): the
  standard does not say which layer colours the body, so it is ambiguous.
- `PROFILE_SET`: an `IfcBeam` with an `IfcMaterialProfileSetUsage` of a
  one-profile set (`Green`).
- `CONSTITUENT_FRAME`: an `IfcMaterialConstituentSet` (`Frame` `Green`,
  `Glazing` `Glass`); the one extrusion is the `IfcShapeAspect` `Frame`,
  so it takes `Green` by item.
- `CONSTITUENT_TWO_PARTS`: the same set; two extrusions, one per aspect,
  so the items differ (`Green`, `Glass`).
- `TYPE_MATERIAL`: no material of its own; its
  `IfcBuildingElementProxyType` carries `Green`.
- `PLAIN`: an `IfcMaterial` (`Plain`) with no representation: no colour.

`material_colour_ifc2x3.ifc` (IFC2X3 TC1) states the releases' own
layout: styled items through `IfcPresentationStyleAssignment`, owner
history on every rooted entity, and `IfcMaterialList` instead of
constituent sets. Products:

- `MATERIAL`, `LAYER_SET`, `ITEM_OVER_MATERIAL` and `MULTI_LAYER` as
  above.
- `MATERIAL_LIST`: an `IfcMaterialList` of one material (`Green`).

Run:  python3 tools/gen_colour_fixtures.py test/fixtures/synthetic-colour
"""

import logging
import pathlib
import sys
import uuid

import ifcopenshell
import ifcopenshell.guid
import ifcopenshell.validate

COLOURS = {
    "Red": ((0.8, 0.1, 0.1), 0.25),
    "Green": ((0.1, 0.7, 0.2), 0.0),
    "Glass": ((0.6, 0.8, 0.9), 0.6),
    "Blue": ((0.1, 0.2, 0.9), 0.0),
    "Yellow": ((0.9, 0.8, 0.1), 0.0),
}


class Builder:
    def __init__(self, schema, label):
        self.f = ifcopenshell.file(schema=schema)
        self.schema = schema
        self.label = label
        self.ifc2x3 = schema == "IFC2X3"
        self.owner = self._owner_history() if self.ifc2x3 else None
        self.styles = {}

    def guid(self, name):
        """A GUID derived from a label, so regeneration is byte-stable."""
        return ifcopenshell.guid.compress(
            uuid.uuid5(uuid.NAMESPACE_URL,
                       f"openbimrs/ifc#449/{self.label}/{name}").hex)

    def _owner_history(self):
        f = self.f
        person = f.create_entity("IfcPerson", FamilyName="Synthetic")
        organization = f.create_entity("IfcOrganization", Name="openbimrs")
        user = f.create_entity("IfcPersonAndOrganization", ThePerson=person,
                               TheOrganization=organization)
        application = f.create_entity(
            "IfcApplication", ApplicationDeveloper=organization, Version="1",
            ApplicationFullName="gen_colour_fixtures.py",
            ApplicationIdentifier="gen_colour_fixtures")
        return f.create_entity("IfcOwnerHistory", OwningUser=user,
                               OwningApplication=application,
                               ChangeAction="NOCHANGE", CreationDate=0)

    def rooted(self, entity, name, **attributes):
        if self.ifc2x3:
            attributes["OwnerHistory"] = self.owner
        return self.f.create_entity(entity, GlobalId=self.guid(name), **attributes)

    def point(self, *coordinates):
        return self.f.create_entity("IfcCartesianPoint",
                                    Coordinates=[float(c) for c in coordinates])

    def placement(self, relative_to=None, x=0.0):
        axis = self.f.create_entity("IfcAxis2Placement3D",
                                    Location=self.point(x, 0.0, 0.0))
        return self.f.create_entity("IfcLocalPlacement", PlacementRelTo=relative_to,
                                    RelativePlacement=axis)

    def surface_style(self, name):
        """One `IfcSurfaceStyle` per colour name, shared by every use."""
        if name in self.styles:
            return self.styles[name]
        (red, green, blue), transparency = COLOURS[name]
        f = self.f
        colour = f.create_entity("IfcColourRgb", Name=name, Red=red, Green=green,
                                 Blue=blue)
        rendering = f.create_entity("IfcSurfaceStyleRendering", SurfaceColour=colour,
                                    Transparency=transparency,
                                    ReflectanceMethod="NOTDEFINED")
        style = f.create_entity("IfcSurfaceStyle", Name=name, Side="BOTH",
                                Styles=[rendering])
        self.styles[name] = style
        return style

    def styles_of(self, name):
        """`IfcStyledItem.Styles` for one surface style, per release."""
        style = self.surface_style(name)
        if self.ifc2x3:
            return [self.f.create_entity("IfcPresentationStyleAssignment",
                                         Styles=[style])]
        return [style]

    def material(self, name, styled=True):
        f = self.f
        material = f.create_entity("IfcMaterial", Name=name)
        if styled:
            item = f.create_entity("IfcStyledItem", Item=None,
                                   Styles=self.styles_of(name))
            representation = f.create_entity(
                "IfcStyledRepresentation", ContextOfItems=self.context,
                RepresentationIdentifier="Style", RepresentationType="Material",
                Items=[item])
            f.create_entity("IfcMaterialDefinitionRepresentation",
                            Representations=[representation],
                            RepresentedMaterial=material)
        return material

    def cube(self, x=0.0, y=0.0, z=0.0, size=1.0):
        f = self.f
        profile = f.create_entity(
            "IfcRectangleProfileDef", ProfileType="AREA",
            Position=f.create_entity("IfcAxis2Placement2D",
                                     Location=f.create_entity(
                                         "IfcCartesianPoint", Coordinates=[0.0, 0.0])),
            XDim=size, YDim=size)
        position = f.create_entity("IfcAxis2Placement3D",
                                   Location=self.point(x, y, z))
        direction = f.create_entity("IfcDirection", DirectionRatios=[0.0, 0.0, 1.0])
        return f.create_entity("IfcExtrudedAreaSolid", SweptArea=profile,
                               Position=position, ExtrudedDirection=direction,
                               Depth=size)

    def body(self, items):
        return self.f.create_entity(
            "IfcShapeRepresentation", ContextOfItems=self.body_context,
            RepresentationIdentifier="Body", RepresentationType="SweptSolid",
            Items=items)

    def product(self, entity, name, items, x, **attributes):
        representation = self.body(items)
        shape = self.f.create_entity("IfcProductDefinitionShape",
                                     Representations=[representation])
        product = self.rooted(entity, name, Name=name,
                              ObjectPlacement=self.placement(self.storey_place, x),
                              Representation=shape, **attributes)
        self.contained.append(product)
        return product, shape, representation

    def associate(self, name, objects, material):
        self.rooted("IfcRelAssociatesMaterial", f"material-{name}",
                    RelatedObjects=objects, RelatingMaterial=material)

    def layer_set(self, name, materials):
        f = self.f
        layers = [f.create_entity("IfcMaterialLayer", Material=material,
                                  LayerThickness=0.5)
                  for material in materials]
        layer_set = f.create_entity("IfcMaterialLayerSet", MaterialLayers=layers,
                                    LayerSetName=name)
        return f.create_entity("IfcMaterialLayerSetUsage", ForLayerSet=layer_set,
                               LayerSetDirection="AXIS2", DirectionSense="POSITIVE",
                               OffsetFromReferenceLine=0.0)

    def spatial(self, project_name):
        f = self.f
        units = f.create_entity("IfcUnitAssignment", Units=[
            f.create_entity("IfcSIUnit", UnitType="LENGTHUNIT", Name="METRE"),
            f.create_entity("IfcSIUnit", UnitType="PLANEANGLEUNIT", Name="RADIAN"),
        ])
        self.context = f.create_entity(
            "IfcGeometricRepresentationContext", ContextType="Model",
            CoordinateSpaceDimension=3, Precision=1.0e-5,
            WorldCoordinateSystem=f.create_entity(
                "IfcAxis2Placement3D", Location=self.point(0, 0, 0)))
        self.body_context = f.create_entity(
            "IfcGeometricRepresentationSubContext", ContextIdentifier="Body",
            ContextType="Model", ParentContext=self.context, TargetView="MODEL_VIEW")
        self.project = self.rooted("IfcProject", "project", Name=project_name,
                                   UnitsInContext=units,
                                   RepresentationContexts=[self.context])
        site_place = self.placement()
        building_place = self.placement(site_place)
        self.storey_place = self.placement(building_place)
        extra = {"CompositionType": "ELEMENT"} if self.ifc2x3 else {}
        site = self.rooted("IfcSite", "site", Name="Site", ObjectPlacement=site_place,
                           **extra)
        building = self.rooted("IfcBuilding", "building", Name="Building",
                               ObjectPlacement=building_place, **extra)
        self.storey = self.rooted("IfcBuildingStorey", "storey", Name="Storey",
                                  ObjectPlacement=self.storey_place, Elevation=0.0,
                                  **extra)
        for parent, child in [(self.project, site), (site, building),
                              (building, self.storey)]:
            self.rooted("IfcRelAggregates", f"aggregates-{child.Name}",
                        RelatingObject=parent, RelatedObjects=[child])
        self.contained = []

    def finish(self):
        self.rooted("IfcRelContainedInSpatialStructure", "contained",
                    RelatingStructure=self.storey, RelatedElements=self.contained)


def build_ifc4(schema):
    b = Builder(schema, schema)
    f = b.f
    b.spatial(f"#449 material colour ({schema})")
    red = b.material("Red")
    green = b.material("Green")
    glass = b.material("Glass")
    plain = b.material("Plain", styled=False)
    proxy = "IfcBuildingElementProxy"

    material_only, _, _ = b.product(proxy, "MATERIAL", [b.cube()], 0.0)
    b.associate("MATERIAL", [material_only], red)

    layered, _, _ = b.product("IfcWall", "LAYER_SET", [b.cube()], 2.0)
    b.associate("LAYER_SET", [layered], b.layer_set("Green layer", [green]))

    styled_cube = b.cube()
    item_over, _, _ = b.product(proxy, "ITEM_OVER_MATERIAL", [styled_cube], 4.0)
    f.create_entity("IfcStyledItem", Item=styled_cube, Styles=b.styles_of("Blue"))
    b.associate("ITEM_OVER_MATERIAL", [item_over], red)

    layered_cube = b.cube()
    layer_over, _, _ = b.product(proxy, "LAYER_OVER_MATERIAL", [layered_cube], 6.0)
    f.create_entity("IfcPresentationLayerWithStyle", Name="A-YELLOW",
                    AssignedItems=[layered_cube], LayerOn=True, LayerFrozen=False,
                    LayerBlocked=False, LayerStyles=[b.surface_style("Yellow")])
    b.associate("LAYER_OVER_MATERIAL", [layer_over], red)

    multi, _, _ = b.product("IfcWall", "MULTI_LAYER", [b.cube()], 8.0)
    b.associate("MULTI_LAYER", [multi], b.layer_set("Red and green", [red, green]))

    beam, _, _ = b.product("IfcBeam", "PROFILE_SET", [b.cube()], 10.0)
    section = f.create_entity(
        "IfcRectangleProfileDef", ProfileType="AREA", ProfileName="1x1", XDim=1.0,
        YDim=1.0)
    profile_set = f.create_entity(
        "IfcMaterialProfileSet", Name="Green beam", MaterialProfiles=[
            f.create_entity("IfcMaterialProfile", Material=green, Profile=section)])
    b.associate("PROFILE_SET", [beam], f.create_entity(
        "IfcMaterialProfileSetUsage", ForProfileSet=profile_set, CardinalPoint=5))

    constituents = f.create_entity(
        "IfcMaterialConstituentSet", Name="Window", MaterialConstituents=[
            f.create_entity("IfcMaterialConstituent", Name="Frame", Material=green),
            f.create_entity("IfcMaterialConstituent", Name="Glazing", Material=glass),
        ])

    def aspect(shape, name, items):
        f.create_entity(
            "IfcShapeAspect", Name=name, ProductDefinitional=True,
            PartOfProductDefinitionShape=shape,
            ShapeRepresentations=[b.body(items)])

    frame_cube = b.cube()
    frame, frame_shape, _ = b.product(proxy, "CONSTITUENT_FRAME", [frame_cube], 12.0)
    aspect(frame_shape, "Frame", [frame_cube])

    part_frame = b.cube(size=0.5)
    part_glass = b.cube(x=0.5, size=0.5)
    two, two_shape, _ = b.product(proxy, "CONSTITUENT_TWO_PARTS",
                                  [part_frame, part_glass], 14.0)
    aspect(two_shape, "Frame", [part_frame])
    aspect(two_shape, "Glazing", [part_glass])
    b.associate("CONSTITUENTS", [frame, two], constituents)

    typed, _, _ = b.product(proxy, "TYPE_MATERIAL", [b.cube()], 16.0)
    proxy_type = b.rooted("IfcBuildingElementProxyType", "proxy-type",
                          Name="Green type", PredefinedType="NOTDEFINED")
    b.rooted("IfcRelDefinesByType", "typed", RelatedObjects=[typed],
             RelatingType=proxy_type)
    b.rooted("IfcRelDeclares", "declares", RelatingContext=b.project,
             RelatedDefinitions=[proxy_type])
    b.associate("TYPE_MATERIAL", [proxy_type], green)

    plain_product, _, _ = b.product(proxy, "PLAIN", [b.cube()], 18.0)
    b.associate("PLAIN", [plain_product], plain)

    b.finish()
    return f


def build_ifc2x3():
    b = Builder("IFC2X3", "IFC2X3")
    f = b.f
    b.spatial("#449 material colour (IFC2X3)")
    red = b.material("Red")
    green = b.material("Green")
    proxy = "IfcBuildingElementProxy"

    material_only, _, _ = b.product(proxy, "MATERIAL", [b.cube()], 0.0)
    b.associate("MATERIAL", [material_only], red)

    layered, _, _ = b.product("IfcWall", "LAYER_SET", [b.cube()], 2.0)
    b.associate("LAYER_SET", [layered], b.layer_set("Green layer", [green]))

    styled_cube = b.cube()
    item_over, _, _ = b.product(proxy, "ITEM_OVER_MATERIAL", [styled_cube], 4.0)
    f.create_entity("IfcStyledItem", Item=styled_cube, Styles=b.styles_of("Blue"))
    b.associate("ITEM_OVER_MATERIAL", [item_over], red)

    multi, _, _ = b.product("IfcWall", "MULTI_LAYER", [b.cube()], 8.0)
    b.associate("MULTI_LAYER", [multi], b.layer_set("Red and green", [red, green]))

    listed, _, _ = b.product(proxy, "MATERIAL_LIST", [b.cube()], 10.0)
    b.associate("MATERIAL_LIST", [listed],
                f.create_entity("IfcMaterialList", Materials=[green]))

    b.finish()
    return f


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
    for product in model.by_type("IfcProduct"):
        if product.Representation is not None:
            print(f"        #{product.id()} {product.Name}")


def main():
    if len(sys.argv) != 2:
        print("usage: gen_colour_fixtures.py <outdir>", file=sys.stderr)
        return 2
    out = pathlib.Path(sys.argv[1])
    out.mkdir(parents=True, exist_ok=True)
    description = "openbimrs/ifc #449: synthetic surface-colour fixture, AGPL-3.0-or-later"
    write(build_ifc4("IFC4"), out, "material_colour.ifc", description)
    write(build_ifc4("IFC4X3_ADD2"), out, "material_colour_ifc4x3.ifc", description)
    write(build_ifc2x3(), out, "material_colour_ifc2x3.ifc", description)
    return 0


if __name__ == "__main__":
    sys.exit(main())
