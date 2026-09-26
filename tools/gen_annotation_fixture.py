#!/usr/bin/env python3
"""Generate the downloadable annotation round-trip fixture (DOC-007, #9).

The 2D approval-plan guide (docs/use-cases/2d-approval-plans.md) promises that
annotation, presentation and library data survive a read/edit/write cycle and
read back through typed views. This file is the example a reader downloads to
check that claim. It is published from `docs/public/fixtures/`, and
`openbim-ifc/tests/docs_examples.rs` reads the published file and asserts the
entities the guide names.

One fire-wall symbol, the kind a German permit plan carries:

| Entity                               | Content                               |
| ------------------------------------ | ------------------------------------- |
| IfcAnnotation `Brandwand`            | contained in the storey, placed in it |
| IfcShapeRepresentation `Annotation2D` | in a PLAN_VIEW `Annotation` subcontext |
| IfcPolyline                          | the wall line, 6000 mm                |
| IfcTextLiteralWithExtent             | `Brandwand F90`, 2000 x 300 mm box     |
| IfcCurveStyle + IfcStyledItem        | red, 50 mm wide                       |
| IfcTextStyle + IfcStyledItem         | Arial 250 mm                          |
| IfcPresentationLayerAssignment       | layer `A-ANNO-FIRE`                   |
| IfcLibraryReference + IfcRelAssociatesLibrary | symbol `BW-F90`              |

Synthetic: written by this script, containing no third-party or normative
material. Licensed with the rest of the repository (AGPL-3.0-or-later).
Millimetres. Entity ids, GUIDs and the header timestamp are fixed so
regenerating produces the same bytes, and the output is checked with
`ifcopenshell.validate` before it is written.

Run:  python3 tools/gen_annotation_fixture.py docs/public/fixtures
"""

import logging
import pathlib
import sys
import uuid

import ifcopenshell
import ifcopenshell.guid
import ifcopenshell.validate

SCHEMA = "IFC4"
NAME = "annotation-plan.ifc"


def guid(label):
    """A GUID derived from a label, so regeneration is byte-stable."""
    return ifcopenshell.guid.compress(
        uuid.uuid5(uuid.NAMESPACE_URL, "openbimrs/ifc#9/" + label).hex)


def point2(f, x, y):
    return f.create_entity("IfcCartesianPoint", Coordinates=[float(x), float(y)])


def placement3(f, relative_to=None):
    origin = f.create_entity("IfcCartesianPoint", Coordinates=[0.0, 0.0, 0.0])
    axis = f.create_entity("IfcAxis2Placement3D", Location=origin)
    return f.create_entity("IfcLocalPlacement", PlacementRelTo=relative_to,
                           RelativePlacement=axis)


def build():
    f = ifcopenshell.file(schema=SCHEMA)

    units = f.create_entity("IfcUnitAssignment", Units=[
        f.create_entity("IfcSIUnit", UnitType="LENGTHUNIT", Prefix="MILLI", Name="METRE"),
        f.create_entity("IfcSIUnit", UnitType="AREAUNIT", Name="SQUARE_METRE"),
        f.create_entity("IfcSIUnit", UnitType="PLANEANGLEUNIT", Name="RADIAN"),
    ])
    plan = f.create_entity(
        "IfcGeometricRepresentationContext", ContextIdentifier="Plan",
        ContextType="Plan", CoordinateSpaceDimension=2, Precision=1.0e-5,
        WorldCoordinateSystem=f.create_entity(
            "IfcAxis2Placement2D", Location=point2(f, 0, 0)))
    annotation_context = f.create_entity(
        "IfcGeometricRepresentationSubContext", ContextIdentifier="Annotation",
        ContextType="Plan", ParentContext=plan, TargetScale=0.01,
        TargetView="PLAN_VIEW")

    project = f.create_entity("IfcProject", GlobalId=guid("project"),
                              Name="Annotation round trip", UnitsInContext=units,
                              RepresentationContexts=[plan])
    site_place = placement3(f)
    building_place = placement3(f, site_place)
    storey_place = placement3(f, building_place)
    site = f.create_entity("IfcSite", GlobalId=guid("site"), Name="Site",
                           ObjectPlacement=site_place)
    building = f.create_entity("IfcBuilding", GlobalId=guid("building"),
                               Name="Building", ObjectPlacement=building_place)
    storey = f.create_entity("IfcBuildingStorey", GlobalId=guid("storey"),
                             Name="EG", ObjectPlacement=storey_place, Elevation=0.0)
    for parent, child in [(project, site), (site, building), (building, storey)]:
        f.create_entity("IfcRelAggregates", GlobalId=guid(f"aggregates-{child.Name}"),
                        RelatingObject=parent, RelatedObjects=[child])

    line = f.create_entity("IfcPolyline", Points=[point2(f, 0, 0), point2(f, 6000, 0)])
    text = f.create_entity(
        "IfcTextLiteralWithExtent", Literal="Brandwand F90",
        Placement=f.create_entity("IfcAxis2Placement2D", Location=point2(f, 3000, 200)),
        Path="RIGHT",
        Extent=f.create_entity("IfcPlanarExtent", SizeInX=2000.0, SizeInY=300.0),
        BoxAlignment="bottom-left")
    representation = f.create_entity(
        "IfcShapeRepresentation", ContextOfItems=annotation_context,
        RepresentationIdentifier="Annotation", RepresentationType="Annotation2D",
        Items=[line, text])

    red = f.create_entity("IfcColourRgb", Name="red", Red=1.0, Green=0.0, Blue=0.0)
    curve_style = f.create_entity(
        "IfcCurveStyle", Name="Brandwand",
        CurveWidth=f.create_entity("IfcPositiveLengthMeasure", 50.0),
        CurveColour=red, ModelOrDraughting=False)
    f.create_entity("IfcStyledItem", Item=line, Styles=[curve_style])
    text_style = f.create_entity(
        "IfcTextStyle", Name="Beschriftung",
        TextCharacterAppearance=f.create_entity("IfcTextStyleForDefinedFont", Colour=red),
        TextFontStyle=f.create_entity(
            "IfcTextStyleFontModel", Name="Arial", FontFamily=["Arial"],
            FontSize=f.create_entity("IfcLengthMeasure", 250.0)),
        ModelOrDraughting=False)
    f.create_entity("IfcStyledItem", Item=text, Styles=[text_style])
    f.create_entity("IfcPresentationLayerAssignment", Name="A-ANNO-FIRE",
                    Description="Fire-protection annotation",
                    AssignedItems=[representation], Identifier="A-ANNO")

    annotation = f.create_entity(
        "IfcAnnotation", GlobalId=guid("brandwand"), Name="Brandwand",
        ObjectPlacement=placement3(f, storey_place),
        Representation=f.create_entity("IfcProductDefinitionShape",
                                       Representations=[representation]))
    f.create_entity("IfcRelContainedInSpatialStructure", GlobalId=guid("contained"),
                    RelatingStructure=storey, RelatedElements=[annotation])

    library = f.create_entity("IfcLibraryInformation", Name="Brandschutz-Symbole",
                              Version="1")
    symbol = f.create_entity("IfcLibraryReference", Identification="BW-F90",
                             Name="Brandwand F90", ReferencedLibrary=library)
    f.create_entity("IfcRelAssociatesLibrary", GlobalId=guid("symbol"),
                    RelatedObjects=[annotation], RelatingLibrary=symbol)
    return f


def main():
    if len(sys.argv) != 2:
        print("usage: gen_annotation_fixture.py <outdir>", file=sys.stderr)
        return 2
    out = pathlib.Path(sys.argv[1])
    out.mkdir(parents=True, exist_ok=True)
    model = build()
    model.header.file_name.name = NAME
    model.header.file_description.description = (
        "openbimrs/ifc DOC-007: synthetic annotation round-trip fixture, "
        "AGPL-3.0-or-later",)
    # A fixed timestamp keeps regeneration byte-stable.
    model.header.file_name.time_stamp = "1970-01-01T00:00:00"

    logger = ifcopenshell.validate.json_logger()
    ifcopenshell.validate.validate(model, logger, express_rules=True)
    if logger.statements:
        for statement in logger.statements:
            print(statement, file=sys.stderr)
        return 1
    logging.getLogger().handlers.clear()

    target = out / NAME
    model.write(str(target))
    print(f"{target.stat().st_size:>7} {NAME}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
