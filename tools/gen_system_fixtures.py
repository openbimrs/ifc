#!/usr/bin/env python3
"""Generate a project-owned fixture stating distribution systems.

The committed corpus contains no IfcRelAssignsToGroup and no system entity of
any kind, so system reading could only ever be exercised against hand-built
in-memory models. That is the same blind spot that let profile coverage be
overstated: a corpus-shaped measure reports what the corpus happens to hold.

Run: python3 tools/gen_system_fixtures.py <output-dir>
"""

import sys
import uuid
from pathlib import Path

import ifcopenshell


def guid():
    return ifcopenshell.guid.compress(uuid.uuid4().hex)


def build():
    f = ifcopenshell.file(schema="IFC4")

    person = f.create_entity("IfcPerson", FamilyName="Test")
    org = f.create_entity("IfcOrganization", Name="openbim")
    pando = f.create_entity("IfcPersonAndOrganization", ThePerson=person, TheOrganization=org)
    app = f.create_entity(
        "IfcApplication", ApplicationDeveloper=org,
        Version="1", ApplicationFullName="gen_system_fixtures", ApplicationIdentifier="gsf")
    owner = f.create_entity(
        "IfcOwnerHistory", OwningUser=pando, OwningApplication=app, ChangeAction="ADDED", CreationDate=0)

    length = f.create_entity("IfcSIUnit", UnitType="LENGTHUNIT", Name="METRE")
    units = f.create_entity("IfcUnitAssignment", Units=[length])
    ctx = f.create_entity(
        "IfcGeometricRepresentationContext",
        ContextType="Model", CoordinateSpaceDimension=3, Precision=1e-5,
        WorldCoordinateSystem=f.create_entity(
            "IfcAxis2Placement3D",
            Location=f.create_entity("IfcCartesianPoint", Coordinates=[0.0, 0.0, 0.0])))
    project = f.create_entity(
        "IfcProject", GlobalId=guid(), OwnerHistory=owner, Name="Systems",
        UnitsInContext=units, RepresentationContexts=[ctx])

    building = f.create_entity(
        "IfcBuilding", GlobalId=guid(), OwnerHistory=owner, Name="Plant")
    f.create_entity(
        "IfcRelAggregates", GlobalId=guid(), OwnerHistory=owner,
        RelatingObject=project, RelatedObjects=[building])

    # Two distribution systems, so a reader that returns "the" system fails.
    heating = f.create_entity(
        "IfcDistributionSystem", GlobalId=guid(), OwnerHistory=owner,
        Name="Heating", LongName="Hot water heating", PredefinedType="HEATING")
    ventilation = f.create_entity(
        "IfcDistributionSystem", GlobalId=guid(), OwnerHistory=owner,
        Name="Ventilation", PredefinedType="VENTILATION")

    # A zone: IfcZone -> IfcSystem in IFC4, so it must be discovered too.
    zone = f.create_entity(
        "IfcZone", GlobalId=guid(), OwnerHistory=owner, Name="Fire compartment")

    # An inventory: an IfcGroup that is NOT a system.
    inventory = f.create_entity(
        "IfcInventory", GlobalId=guid(), OwnerHistory=owner, Name="Spares",
        PredefinedType="FURNITUREINVENTORY")

    segments = [
        f.create_entity("IfcFlowSegment", GlobalId=guid(), OwnerHistory=owner,
                        Name=f"Pipe {i}")
        for i in range(3)
    ]
    fitting = f.create_entity(
        "IfcFlowFitting", GlobalId=guid(), OwnerHistory=owner, Name="Elbow")
    terminal = f.create_entity(
        "IfcFlowTerminal", GlobalId=guid(), OwnerHistory=owner, Name="Radiator")

    f.create_entity(
        "IfcRelAssignsToGroup", GlobalId=guid(), OwnerHistory=owner,
        RelatedObjects=[segments[0], segments[1], fitting, terminal],
        RelatingGroup=heating)
    f.create_entity(
        "IfcRelAssignsToGroup", GlobalId=guid(), OwnerHistory=owner,
        RelatedObjects=[segments[2]], RelatingGroup=ventilation)
    # Assignment to a non-system group, which must not become a membership.
    f.create_entity(
        "IfcRelAssignsToGroup", GlobalId=guid(), OwnerHistory=owner,
        RelatedObjects=[fitting], RelatingGroup=inventory)

    # Services relationship: slots are RelatingSystem=4, RelatedBuildings=5,
    # the reverse of RelAssignsToGroup.
    f.create_entity(
        "IfcRelServicesBuildings", GlobalId=guid(), OwnerHistory=owner,
        RelatingSystem=heating, RelatedBuildings=[building])

    return f, zone


def main():
    out = Path(sys.argv[1] if len(sys.argv) > 1 else ".")
    out.mkdir(parents=True, exist_ok=True)
    f, _ = build()
    path = out / "synthetic_systems.ifc"
    f.write(str(path))
    print(f"wrote {path}")


if __name__ == "__main__":
    main()
