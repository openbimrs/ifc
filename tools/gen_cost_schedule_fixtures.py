#!/usr/bin/env python3
"""Generate the cost/schedule fixture for ifc-cost and ifc-schedule tests.

Regenerate with:

    python3 tools/gen_cost_schedule_fixtures.py test/fixtures/synthetic-cost-schedule

The file states a small refurbishment: a cost schedule whose items nest into a
breakdown, and a work schedule whose tasks sequence with lag. Two facts are
deliberately contradictory so the crates have something real to report:

  * a milestone task that also states a schedule duration (IfcTaskTime WR1);
  * a cost item whose own stated value disagrees with the sum of its children.

Both are legal STEP and pass IfcOpenShell validation -- they are semantic
contradictions, not syntax errors, which is exactly the class of problem a
reader has to surface itself.
"""

import argparse
import pathlib
import sys
import uuid

import ifcopenshell


def guid() -> str:
    return ifcopenshell.guid.compress(uuid.uuid4().hex)


def build() -> ifcopenshell.file:
    f = ifcopenshell.file(schema="IFC4")

    person = f.create_entity("IfcPerson", FamilyName="Schroedter")
    org = f.create_entity("IfcOrganization", Name="openbim")
    person_org = f.create_entity(
        "IfcPersonAndOrganization", ThePerson=person, TheOrganization=org
    )
    app = f.create_entity(
        "IfcApplication",
        ApplicationDeveloper=org,
        Version="0.1.0",
        ApplicationFullName="openbim fixture generator",
        ApplicationIdentifier="openbim",
    )
    owner = f.create_entity(
        "IfcOwnerHistory",
        OwningUser=person_org,
        OwningApplication=app,
        ChangeAction="NOCHANGE",
        CreationDate=0,
    )

    # --- units, including exactly one currency ---------------------------
    length = f.create_entity("IfcSIUnit", UnitType="LENGTHUNIT", Name="METRE")
    area = f.create_entity("IfcSIUnit", UnitType="AREAUNIT", Name="SQUARE_METRE")
    volume = f.create_entity("IfcSIUnit", UnitType="VOLUMEUNIT", Name="CUBIC_METRE")
    time_unit = f.create_entity("IfcSIUnit", UnitType="TIMEUNIT", Name="SECOND")
    # IFC4 states Currency as IfcLabel, so this is a quoted string.
    currency = f.create_entity("IfcMonetaryUnit", Currency="EUR")
    units = f.create_entity(
        "IfcUnitAssignment", Units=[length, area, volume, time_unit, currency]
    )

    ctx = f.create_entity(
        "IfcGeometricRepresentationContext",
        ContextType="Model",
        CoordinateSpaceDimension=3,
        Precision=1e-5,
        WorldCoordinateSystem=f.create_entity(
            "IfcAxis2Placement3D",
            Location=f.create_entity(
                "IfcCartesianPoint", Coordinates=[0.0, 0.0, 0.0]
            ),
        ),
    )
    project = f.create_entity(
        "IfcProject",
        GlobalId=guid(),
        OwnerHistory=owner,
        Name="Refurbishment",
        RepresentationContexts=[ctx],
        UnitsInContext=units,
    )

    site = f.create_entity(
        "IfcSite", GlobalId=guid(), OwnerHistory=owner, Name="Site",
        CompositionType="ELEMENT",
    )
    building = f.create_entity(
        "IfcBuilding", GlobalId=guid(), OwnerHistory=owner, Name="Building",
        CompositionType="ELEMENT",
    )
    storey = f.create_entity(
        "IfcBuildingStorey", GlobalId=guid(), OwnerHistory=owner,
        Name="Ground floor", CompositionType="ELEMENT",
    )
    for parent, children in (
        (project, [site]),
        (site, [building]),
        (building, [storey]),
    ):
        f.create_entity(
            "IfcRelAggregates",
            GlobalId=guid(),
            OwnerHistory=owner,
            RelatingObject=parent,
            RelatedObjects=children,
        )

    # --- the products being costed and built -----------------------------
    wall = f.create_entity(
        "IfcWall", GlobalId=guid(), OwnerHistory=owner, Name="Party wall"
    )
    slab = f.create_entity(
        "IfcSlab", GlobalId=guid(), OwnerHistory=owner, Name="Ground slab"
    )
    f.create_entity(
        "IfcRelContainedInSpatialStructure",
        GlobalId=guid(),
        OwnerHistory=owner,
        RelatingStructure=storey,
        RelatedElements=[wall, slab],
    )

    # --- quantities the costs are computed against -----------------------
    wall_volume = f.create_entity(
        "IfcQuantityVolume", Name="NetVolume", Unit=volume, VolumeValue=12.5
    )
    slab_area = f.create_entity(
        "IfcQuantityArea", Name="NetArea", Unit=area, AreaValue=48.0
    )
    for element, quantity in ((wall, wall_volume), (slab, slab_area)):
        qset = f.create_entity(
            "IfcElementQuantity",
            GlobalId=guid(),
            OwnerHistory=owner,
            Name="BaseQuantities",
            Quantities=[quantity],
        )
        f.create_entity(
            "IfcRelDefinesByProperties",
            GlobalId=guid(),
            OwnerHistory=owner,
            RelatedObjects=[element],
            RelatingPropertyDefinition=qset,
        )

    # --- cost -------------------------------------------------------------
    cost_schedule = f.create_entity(
        "IfcCostSchedule",
        GlobalId=guid(),
        OwnerHistory=owner,
        Name="Budget",
        Identification="CS-1",
        PredefinedType="BUDGET",
        Status="Draft",
        SubmittedOn="2026-01-15T09:00:00",
    )

    def monetary(amount: float):
        return f.create_entity("IfcMonetaryMeasure", amount)

    # A rate: 45.50 EUR per 1 cubic metre. UnitBasis is what makes it a rate
    # rather than a lump sum.
    rate_basis = f.create_entity(
        "IfcMeasureWithUnit",
        ValueComponent=f.create_entity("IfcVolumeMeasure", 1.0),
        UnitComponent=volume,
    )
    concrete_rate = f.create_entity(
        "IfcCostValue",
        Name="Concrete rate",
        AppliedValue=monetary(45.50),
        UnitBasis=rate_basis,
        Category="Material",
    )
    labour_value = f.create_entity(
        "IfcCostValue",
        Name="Labour",
        AppliedValue=monetary(320.00),
        Category="Labour",
    )
    plant_value = f.create_entity(
        "IfcCostValue",
        Name="Plant",
        AppliedValue=monetary(180.00),
        Category="Plant",
    )
    # A COMPOSED value: no AppliedValue of its own, only components and an
    # operator. A reader that only looks at slot 2 reports nothing for this.
    composed = f.create_entity(
        "IfcCostValue",
        Name="Site setup",
        Category="Preliminaries",
        ArithmeticOperator="ADD",
        Components=[labour_value, plant_value],
    )

    # Parent states 500.00; children state 320 + 180 = 500.00 -> consistent.
    substructure = f.create_entity(
        "IfcCostItem",
        GlobalId=guid(),
        OwnerHistory=owner,
        Name="Substructure",
        Identification="A.1",
        PredefinedType="USERDEFINED",
        ObjectType="Group",
        CostValues=[f.create_entity(
            "IfcCostValue", Name="Subtotal", AppliedValue=monetary(500.00)
        )],
    )
    excavation = f.create_entity(
        "IfcCostItem",
        GlobalId=guid(),
        OwnerHistory=owner,
        Name="Excavation",
        Identification="A.1.1",
        CostValues=[labour_value],
    )
    concreting = f.create_entity(
        "IfcCostItem",
        GlobalId=guid(),
        OwnerHistory=owner,
        Name="Concreting",
        Identification="A.1.2",
        CostValues=[plant_value],
        CostQuantities=[wall_volume],
    )
    # DELIBERATE inconsistency: states 900.00 but its child totals 45.50.
    superstructure = f.create_entity(
        "IfcCostItem",
        GlobalId=guid(),
        OwnerHistory=owner,
        Name="Superstructure",
        Identification="A.2",
        CostValues=[f.create_entity(
            "IfcCostValue", Name="Subtotal", AppliedValue=monetary(900.00)
        )],
    )
    cladding = f.create_entity(
        "IfcCostItem",
        GlobalId=guid(),
        OwnerHistory=owner,
        Name="Cladding",
        Identification="A.2.1",
        CostValues=[concrete_rate],
        CostQuantities=[slab_area],
    )
    preliminaries = f.create_entity(
        "IfcCostItem",
        GlobalId=guid(),
        OwnerHistory=owner,
        Name="Preliminaries",
        Identification="A.3",
        CostValues=[composed],
    )

    # The schedule holds its top-level items; items nest into a breakdown.
    f.create_entity(
        "IfcRelAssignsToControl",
        GlobalId=guid(),
        OwnerHistory=owner,
        RelatedObjects=[substructure, superstructure, preliminaries],
        RelatingControl=cost_schedule,
    )
    f.create_entity(
        "IfcRelNests",
        GlobalId=guid(),
        OwnerHistory=owner,
        RelatingObject=substructure,
        RelatedObjects=[excavation, concreting],
    )
    f.create_entity(
        "IfcRelNests",
        GlobalId=guid(),
        OwnerHistory=owner,
        RelatingObject=superstructure,
        RelatedObjects=[cladding],
    )
    # Cost items price real products.
    f.create_entity(
        "IfcRelAssignsToControl",
        GlobalId=guid(),
        OwnerHistory=owner,
        RelatedObjects=[wall],
        RelatingControl=concreting,
    )
    f.create_entity(
        "IfcRelAssignsToControl",
        GlobalId=guid(),
        OwnerHistory=owner,
        RelatedObjects=[slab],
        RelatingControl=cladding,
    )

    # --- schedule ---------------------------------------------------------
    calendar = f.create_entity(
        "IfcWorkCalendar",
        GlobalId=guid(),
        OwnerHistory=owner,
        Name="Site calendar",
        Identification="CAL-1",
        PredefinedType="FIRSTSHIFT",
        WorkingTimes=[
            f.create_entity(
                "IfcWorkTime",
                Name="Weekdays",
                Start="2026-03-02",
                Finish="2026-06-30",
                RecurrencePattern=f.create_entity(
                    "IfcRecurrencePattern",
                    RecurrenceType="WEEKLY",
                    # Monday..Friday. Unbounded: no Occurrences stated.
                    WeekdayComponent=[1, 2, 3, 4, 5],
                    Interval=1,
                ),
            )
        ],
        ExceptionTimes=[
            f.create_entity(
                "IfcWorkTime",
                Name="Easter",
                Start="2026-04-03",
                Finish="2026-04-06",
            )
        ],
    )

    work_plan = f.create_entity(
        "IfcWorkPlan",
        GlobalId=guid(),
        OwnerHistory=owner,
        Name="Programme",
        Identification="WP-1",
        CreationDate="2026-01-10T08:00:00",
        StartTime="2026-03-02T08:00:00",
        FinishTime="2026-06-30T17:00:00",
        PredefinedType="PLANNED",
    )
    work_schedule = f.create_entity(
        "IfcWorkSchedule",
        GlobalId=guid(),
        OwnerHistory=owner,
        Name="Construction schedule",
        Identification="WS-1",
        CreationDate="2026-01-10T08:00:00",
        StartTime="2026-03-02T08:00:00",
        FinishTime="2026-06-30T17:00:00",
        PredefinedType="PLANNED",
    )
    f.create_entity(
        "IfcRelAggregates",
        GlobalId=guid(),
        OwnerHistory=owner,
        RelatingObject=work_plan,
        RelatedObjects=[work_schedule],
    )

    def task(name, ident, *, milestone=False, time=None, predefined="CONSTRUCTION"):
        return f.create_entity(
            "IfcTask",
            GlobalId=guid(),
            OwnerHistory=owner,
            Name=name,
            Identification=ident,
            IsMilestone=milestone,
            PredefinedType=predefined,
            TaskTime=time,
        )

    excavate_time = f.create_entity(
        "IfcTaskTime",
        Name="Excavate",
        DurationType="WORKTIME",
        ScheduleDuration="P5D",
        ScheduleStart="2026-03-02T08:00:00",
        ScheduleFinish="2026-03-06T17:00:00",
        IsCritical=True,
    )
    pour_time = f.create_entity(
        "IfcTaskTime",
        Name="Pour",
        DurationType="WORKTIME",
        ScheduleDuration="P3D",
        ScheduleStart="2026-03-09T08:00:00",
        ScheduleFinish="2026-03-11T17:00:00",
        IsCritical=True,
    )
    clad_time = f.create_entity(
        "IfcTaskTime",
        Name="Clad",
        DurationType="WORKTIME",
        ScheduleDuration="P10D",
        ScheduleStart="2026-03-16T08:00:00",
        ScheduleFinish="2026-03-27T17:00:00",
        IsCritical=False,
        FreeFloat="P2D",
        TotalFloat="P4D",
        Completion=25.0,
    )
    # DELIBERATE WR1 contradiction: a milestone that states a duration.
    handover_time = f.create_entity(
        "IfcTaskTime",
        Name="Handover",
        DurationType="ELAPSEDTIME",
        ScheduleDuration="P1D",
        ScheduleStart="2026-06-30T09:00:00",
    )

    excavate = task("Excavate", "T-1", time=excavate_time)
    pour = task("Pour foundations", "T-2", time=pour_time)
    clad = task("Install cladding", "T-3", time=clad_time)
    fitout = task("Fit out", "T-4")
    handover = task(
        "Practical completion", "T-5", milestone=True, time=handover_time
    )

    f.create_entity(
        "IfcRelAssignsToControl",
        GlobalId=guid(),
        OwnerHistory=owner,
        RelatedObjects=[excavate, pour, clad, fitout, handover],
        RelatingControl=work_schedule,
    )
    f.create_entity(
        "IfcRelAssignsToControl",
        GlobalId=guid(),
        OwnerHistory=owner,
        RelatedObjects=[excavate, pour, clad, fitout, handover],
        RelatingControl=calendar,
    )

    def sequence(pred, succ, seq_type, lag=None):
        return f.create_entity(
            "IfcRelSequence",
            GlobalId=guid(),
            OwnerHistory=owner,
            RelatingProcess=pred,
            RelatedProcess=succ,
            SequenceType=seq_type,
            TimeLag=lag,
        )

    # A curing lag: pouring finishes, then 2 days pass before cladding.
    cure_lag = f.create_entity(
        "IfcLagTime",
        Name="Curing",
        LagValue=f.create_entity("IfcDuration", "P2D"),
        DurationType="ELAPSEDTIME",
    )
    sequence(excavate, pour, "FINISH_START")
    sequence(pour, clad, "FINISH_START", cure_lag)
    # A START_START link: fit-out begins with cladding, not after it. A tool
    # assuming finish-to-start everywhere gets this wrong.
    sequence(clad, fitout, "START_START")
    sequence(fitout, handover, "FINISH_START")

    # Tasks nest into a work breakdown as well as sequencing.
    f.create_entity(
        "IfcRelNests",
        GlobalId=guid(),
        OwnerHistory=owner,
        RelatingObject=excavate,
        RelatedObjects=[pour],
    )

    # Task pricing: link the cost item to the task that performs it.
    f.create_entity(
        "IfcRelAssignsToControl",
        GlobalId=guid(),
        OwnerHistory=owner,
        RelatedObjects=[excavate],
        RelatingControl=excavation,
    )

    event_time = f.create_entity(
        "IfcEventTime",
        Name="Inspection",
        ScheduleDate="2026-03-12T10:00:00",
        ActualDate="2026-03-12T14:00:00",
    )
    f.create_entity(
        "IfcEvent",
        GlobalId=guid(),
        OwnerHistory=owner,
        Name="Foundation inspection",
        Identification="E-1",
        PredefinedType="INTERMEDIATEEVENT",
        EventTriggerType="EVENTRULE",
        EventOccurenceTime=event_time,
    )

    return f


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("out", type=pathlib.Path)
    args = parser.parse_args()
    args.out.mkdir(parents=True, exist_ok=True)
    path = args.out / "synthetic_cost_schedule.ifc"
    build().write(str(path))
    print(f"wrote {path}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
