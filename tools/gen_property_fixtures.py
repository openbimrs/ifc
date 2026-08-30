#!/usr/bin/env python3
"""Generate the property/quantity fixture for ifc-properties tests.

Regenerate with:

    python3 tools/gen_property_fixtures.py test/fixtures/synthetic-properties

The file is committed. It exists because no surveyed public corpus states the
combination this crate must read: every property value family, a type/
occurrence override pair, prefixed and conversion-based units, and the
malformed cases the schema forbids but exporters emit.
"""
import sys
import uuid
import pathlib
import ifcopenshell


def guid():
    return ifcopenshell.guid.compress(uuid.uuid4().hex)


def build():
    f = ifcopenshell.file(schema="IFC4")

    person = f.create_entity("IfcPerson", FamilyName="Generator")
    org = f.create_entity("IfcOrganization", Name="openbim.rs")
    p_and_o = f.create_entity(
        "IfcPersonAndOrganization", ThePerson=person, TheOrganization=org)
    app = f.create_entity(
        "IfcApplication", ApplicationDeveloper=org, Version="1",
        ApplicationFullName="gen_property_fixtures", ApplicationIdentifier="gpf")
    owner = f.create_entity(
        "IfcOwnerHistory", OwningUser=p_and_o, OwningApplication=app,
        ChangeAction="ADDED", CreationDate=0)

    # ---- Units -----------------------------------------------------------
    #
    # A prefixed SI unit (millimetre) and a conversion-based one (inch) so the
    # reader is exercised on both. The project length unit is METRE, which is
    # deliberately DIFFERENT from the millimetre a quantity states: a reader
    # that falls back to the project default would misreport it by 1000x.
    metre = f.create_entity("IfcSIUnit", UnitType="LENGTHUNIT", Name="METRE")
    millimetre = f.create_entity(
        "IfcSIUnit", UnitType="LENGTHUNIT", Prefix="MILLI", Name="METRE")
    sq_metre = f.create_entity("IfcSIUnit", UnitType="AREAUNIT", Name="SQUARE_METRE")
    cu_metre = f.create_entity("IfcSIUnit", UnitType="VOLUMEUNIT", Name="CUBIC_METRE")
    kilogram = f.create_entity("IfcSIUnit", UnitType="MASSUNIT", Name="GRAM", Prefix="KILO")
    second = f.create_entity("IfcSIUnit", UnitType="TIMEUNIT", Name="SECOND")

    # inch = 25.4 mm, expressed as a measure with a unit.
    inch_factor = f.create_entity(
        "IfcMeasureWithUnit",
        ValueComponent=f.create_entity("IfcLengthMeasure", 25.4),
        UnitComponent=millimetre)
    inch = f.create_entity(
        "IfcConversionBasedUnit", Dimensions=f.create_entity(
            "IfcDimensionalExponents", 1, 0, 0, 0, 0, 0, 0),
        UnitType="LENGTHUNIT", Name="inch", ConversionFactor=inch_factor)

    # A derived unit: m3/s, two elements with different exponents.
    flow = f.create_entity(
        "IfcDerivedUnit",
        Elements=[
            f.create_entity("IfcDerivedUnitElement", Unit=cu_metre, Exponent=1),
            f.create_entity("IfcDerivedUnitElement", Unit=second, Exponent=-1),
        ],
        UnitType="VOLUMETRICFLOWRATEUNIT")

    units = f.create_entity(
        "IfcUnitAssignment",
        Units=[metre, sq_metre, cu_metre, kilogram, second])

    ctx = f.create_entity(
        "IfcGeometricRepresentationContext", ContextType="Model",
        CoordinateSpaceDimension=3, Precision=1e-5,
        WorldCoordinateSystem=f.create_entity(
            "IfcAxis2Placement3D",
            Location=f.create_entity("IfcCartesianPoint", Coordinates=(0., 0., 0.))))

    project = f.create_entity(
        "IfcProject", GlobalId=guid(), OwnerHistory=owner, Name="Properties",
        UnitsInContext=units, RepresentationContexts=[ctx])

    site = f.create_entity("IfcSite", GlobalId=guid(), OwnerHistory=owner, Name="Site")
    building = f.create_entity(
        "IfcBuilding", GlobalId=guid(), OwnerHistory=owner, Name="Building")
    storey = f.create_entity(
        "IfcBuildingStorey", GlobalId=guid(), OwnerHistory=owner, Name="Level 0")
    f.create_entity("IfcRelAggregates", GlobalId=guid(), OwnerHistory=owner,
                    RelatingObject=project, RelatedObjects=[site])
    f.create_entity("IfcRelAggregates", GlobalId=guid(), OwnerHistory=owner,
                    RelatingObject=site, RelatedObjects=[building])
    f.create_entity("IfcRelAggregates", GlobalId=guid(), OwnerHistory=owner,
                    RelatingObject=building, RelatedObjects=[storey])

    # ---- Elements and their type ----------------------------------------
    wall_type = f.create_entity(
        "IfcWallType", GlobalId=guid(), OwnerHistory=owner, Name="WT-200",
        PredefinedType="SOLIDWALL")
    wall_a = f.create_entity(
        "IfcWall", GlobalId=guid(), OwnerHistory=owner, Name="Wall A")
    wall_b = f.create_entity(
        "IfcWall", GlobalId=guid(), OwnerHistory=owner, Name="Wall B")
    f.create_entity(
        "IfcRelContainedInSpatialStructure", GlobalId=guid(), OwnerHistory=owner,
        RelatingStructure=storey, RelatedElements=[wall_a, wall_b])
    f.create_entity(
        "IfcRelDefinesByType", GlobalId=guid(), OwnerHistory=owner,
        RelatedObjects=[wall_a, wall_b], RelatingType=wall_type)

    # ---- Property values: every family ----------------------------------
    def single(name, value):
        return f.create_entity(
            "IfcPropertySingleValue", Name=name, NominalValue=value)

    # The TYPE states IsExternal=TRUE and FireRating="F30".
    type_pset = f.create_entity(
        "IfcPropertySet", GlobalId=guid(), OwnerHistory=owner,
        Name="Pset_WallCommon",
        HasProperties=[
            single("IsExternal", f.create_entity("IfcBoolean", True)),
            single("FireRating", f.create_entity("IfcLabel", "F30")),
        ])
    # A type holds its sets DIRECTLY: IfcRelDefinesByProperties is forbidden
    # for types by the NoRelatedTypeObject WHERE rule.
    wall_type.HasPropertySets = [type_pset]

    # Wall A OVERRIDES IsExternal with FALSE, in a same-named set. The
    # occurrence must win, and the type value must remain visible as shadowed.
    occ_pset = f.create_entity(
        "IfcPropertySet", GlobalId=guid(), OwnerHistory=owner,
        Name="Pset_WallCommon",
        HasProperties=[
            single("IsExternal", f.create_entity("IfcBoolean", False)),
        ])
    f.create_entity(
        "IfcRelDefinesByProperties", GlobalId=guid(), OwnerHistory=owner,
        RelatedObjects=[wall_a], RelatingPropertyDefinition=occ_pset)

    # Every remaining property family, on Wall B.
    enumeration = f.create_entity(
        "IfcPropertyEnumeration", Name="Colours",
        EnumerationValues=[
            f.create_entity("IfcLabel", "red"),
            f.create_entity("IfcLabel", "green"),
        ])
    families = f.create_entity(
        "IfcPropertySet", GlobalId=guid(), OwnerHistory=owner,
        Name="Pset_Families",
        HasProperties=[
            single("Thickness", f.create_entity("IfcLengthMeasure", 0.2)),
            f.create_entity(
                "IfcPropertyEnumeratedValue", Name="Colour",
                EnumerationValues=[f.create_entity("IfcLabel", "red")],
                EnumerationReference=enumeration),
            # UpperBoundValue is slot 2 and LowerBoundValue slot 3: a reader
            # that swaps them inverts the range and still looks plausible.
            f.create_entity(
                "IfcPropertyBoundedValue", Name="Range",
                UpperBoundValue=f.create_entity("IfcLengthMeasure", 10.0),
                LowerBoundValue=f.create_entity("IfcLengthMeasure", 2.0),
                SetPointValue=f.create_entity("IfcLengthMeasure", 6.0)),
            f.create_entity(
                "IfcPropertyListValue", Name="Layers",
                ListValues=[
                    f.create_entity("IfcLengthMeasure", 0.012),
                    f.create_entity("IfcLengthMeasure", 0.15),
                    f.create_entity("IfcLengthMeasure", 0.012),
                ]),
            f.create_entity(
                "IfcPropertyTableValue", Name="Curve",
                DefiningValues=[
                    f.create_entity("IfcThermodynamicTemperatureMeasure", 0.0),
                    f.create_entity("IfcThermodynamicTemperatureMeasure", 20.0),
                ],
                DefinedValues=[
                    f.create_entity("IfcThermalTransmittanceMeasure", 0.30),
                    f.create_entity("IfcThermalTransmittanceMeasure", 0.24),
                ],
                CurveInterpolation="LINEAR"),
            f.create_entity(
                "IfcPropertyReferenceValue", Name="Material",
                UsageName="Reference", PropertyReference=f.create_entity(
                    "IfcMaterial", Name="Concrete")),
            f.create_entity(
                "IfcComplexProperty", Name="Assembly", UsageName="Layered",
                HasProperties=[
                    single("Core", f.create_entity("IfcLengthMeasure", 0.15)),
                    single("Finish", f.create_entity("IfcLabel", "paint")),
                ]),
        ])
    f.create_entity(
        "IfcRelDefinesByProperties", GlobalId=guid(), OwnerHistory=owner,
        RelatedObjects=[wall_b], RelatingPropertyDefinition=families)

    # ---- Quantities ------------------------------------------------------
    #
    # Length in MILLIMETRE while the project default is METRE: a reader that
    # substitutes the project unit reports 200 metres of wall thickness.
    quantities = f.create_entity(
        "IfcElementQuantity", GlobalId=guid(), OwnerHistory=owner,
        Name="Qto_WallBaseQuantities", MethodOfMeasurement="BaseQuantities",
        Quantities=[
            f.create_entity("IfcQuantityLength", Name="Width",
                            LengthValue=200.0, Unit=millimetre),
            f.create_entity("IfcQuantityArea", Name="NetSideArea",
                            AreaValue=12.5, Unit=sq_metre),
            f.create_entity("IfcQuantityVolume", Name="NetVolume",
                            VolumeValue=2.5, Unit=cu_metre),
            f.create_entity("IfcQuantityCount", Name="Count", CountValue=1.0),
            f.create_entity("IfcQuantityWeight", Name="NetWeight",
                            WeightValue=5750.0, Unit=kilogram),
            f.create_entity("IfcQuantityTime", Name="InstallTime",
                            TimeValue=3600.0, Unit=second),
            f.create_entity(
                "IfcPhysicalComplexQuantity", Name="Layers",
                Discrimination="layer",
                HasQuantities=[
                    f.create_entity("IfcQuantityLength", Name="Inner",
                                    LengthValue=12.0, Unit=millimetre),
                    f.create_entity("IfcQuantityLength", Name="Outer",
                                    LengthValue=12.0, Unit=millimetre),
                ]),
        ])
    f.create_entity(
        "IfcRelDefinesByProperties", GlobalId=guid(), OwnerHistory=owner,
        RelatedObjects=[wall_a], RelatingPropertyDefinition=quantities)

    # ---- A template ------------------------------------------------------
    template = f.create_entity(
        "IfcPropertySetTemplate", GlobalId=guid(), OwnerHistory=owner,
        Name="Pset_Families", TemplateType="PSET_OCCURRENCEDRIVEN",
        ApplicableEntity="IfcWall",
        HasPropertyTemplates=[
            f.create_entity(
                "IfcSimplePropertyTemplate", GlobalId=guid(), OwnerHistory=owner,
                Name="Thickness", TemplateType="P_SINGLEVALUE",
                PrimaryMeasureType="IfcLengthMeasure", PrimaryUnit=metre),
        ])
    f.create_entity(
        "IfcRelDefinesByTemplate", GlobalId=guid(), OwnerHistory=owner,
        RelatedPropertySets=[families], RelatingTemplate=template)

    # The derived unit is referenced so it is not orphaned.
    f.create_entity(
        "IfcPropertySet", GlobalId=guid(), OwnerHistory=owner, Name="Pset_Flow",
        HasProperties=[
            f.create_entity(
                "IfcPropertySingleValue", Name="Flow",
                NominalValue=f.create_entity("IfcVolumetricFlowRateMeasure", 0.05),
                Unit=flow),
        ])
    return f


def main():
    out = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else ".")
    out.mkdir(parents=True, exist_ok=True)
    path = out / "synthetic_properties.ifc"
    build().write(str(path))
    print(f"wrote {path}")


if __name__ == "__main__":
    main()
