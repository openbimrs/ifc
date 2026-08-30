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

    # ---- Ports and connections -------------------------------------------
    #
    # Topology, chosen so a naive reader fails visibly:
    #
    #   seg0 -- seg1 -- fitting -- terminal        (heating, a chain)
    #   seg2 (ventilation, its own component)
    #   ring: seg0/fitting close a LOOP so traversal must not run forever
    #
    # Ports are attached by BOTH mechanisms: IfcRelNests for the IFC4 form and
    # IfcRelConnectsPortToElement for the legacy form still emitted by real
    # exporters. A reader that knows only one silently loses half the ports.
    def port(name, flow):
        return f.create_entity(
            "IfcDistributionPort", GlobalId=guid(), OwnerHistory=owner,
            Name=name, FlowDirection=flow, PredefinedType="PIPE")

    # seg0: nested (IFC4 form).
    seg0_in = port("seg0-in", "SINK")
    seg0_out = port("seg0-out", "SOURCE")
    f.create_entity(
        "IfcRelNests", GlobalId=guid(), OwnerHistory=owner,
        RelatingObject=segments[0], RelatedObjects=[seg0_in, seg0_out])

    # seg1: legacy form, one relationship per port.
    seg1_in = port("seg1-in", "SINK")
    seg1_out = port("seg1-out", "SOURCE")
    for p_ in (seg1_in, seg1_out):
        f.create_entity(
            "IfcRelConnectsPortToElement", GlobalId=guid(), OwnerHistory=owner,
            RelatingPort=p_, RelatedElement=segments[1])

    # fitting: nested, three ports (a tee).
    fit_a = port("fit-a", "SINK")
    fit_b = port("fit-b", "SOURCE")
    fit_c = port("fit-c", "SOURCEANDSINK")
    f.create_entity(
        "IfcRelNests", GlobalId=guid(), OwnerHistory=owner,
        RelatingObject=fitting, RelatedObjects=[fit_a, fit_b, fit_c])

    # terminal: nested, single port.
    term_in = port("term-in", "SINK")
    f.create_entity(
        "IfcRelNests", GlobalId=guid(), OwnerHistory=owner,
        RelatingObject=terminal, RelatedObjects=[term_in])

    # seg2 on the OTHER system: its own disconnected component.
    seg2_in = port("seg2-in", "SINK")
    seg2_out = port("seg2-out", "SOURCE")
    f.create_entity(
        "IfcRelNests", GlobalId=guid(), OwnerHistory=owner,
        RelatingObject=segments[2], RelatedObjects=[seg2_in, seg2_out])

    # An UNATTACHED port: legal, and its element must read as None rather
    # than being dropped from the port list.
    orphan = port("orphan", "NOTDEFINED")

    # Connections. RealizingElement is set on one to prove slot 6 is read.
    f.create_entity(
        "IfcRelConnectsPorts", GlobalId=guid(), OwnerHistory=owner,
        RelatingPort=seg0_out, RelatedPort=seg1_in, RealizingElement=segments[0])
    f.create_entity(
        "IfcRelConnectsPorts", GlobalId=guid(), OwnerHistory=owner,
        RelatingPort=seg1_out, RelatedPort=fit_a)
    f.create_entity(
        "IfcRelConnectsPorts", GlobalId=guid(), OwnerHistory=owner,
        RelatingPort=fit_b, RelatedPort=term_in)
    # Closes a ring: fit_c back to seg0_in. Traversal must terminate.
    f.create_entity(
        "IfcRelConnectsPorts", GlobalId=guid(), OwnerHistory=owner,
        RelatingPort=fit_c, RelatedPort=seg0_in)
    # seg2's own two ports connect to each other only.
    f.create_entity(
        "IfcRelConnectsPorts", GlobalId=guid(), OwnerHistory=owner,
        RelatingPort=seg2_out, RelatedPort=seg2_in)

    _ = orphan

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
