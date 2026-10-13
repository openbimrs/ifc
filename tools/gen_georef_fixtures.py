#!/usr/bin/env python3
"""Generate the IFC2X3 property-set georeferencing fixtures for #451.

Regenerate with:

    python3 tools/gen_georef_fixtures.py test/fixtures/synthetic-georef

IFC2X3 has no IfcMapConversion or IfcProjectedCRS. The buildingSMART
Australasia "User Guide for Geo-referencing in IFC" v2.0 (2020), section 5.1,
Tables 2 to 4, carries the same parameters in two property sets,
ePSet_MapConversion and ePSet_ProjectedCRS, linked to IfcProject. No
licence-clean public corpus pairs such a file with its IFC4 equivalent, so
both are generated; the output is our own data. Fixed GUIDs and time stamp:
regeneration is byte-stable.

The conversion is the worked example of the guide's section 5.2 (MGA zone 56):
Eastings 333780.622, Northings 6246775.891, OrthogonalHeight 97.457,
XAxisAbscissa 0.990330045, XAxisOrdinate -0.138731399, Scale 0.999998, in
metres. Here the project length unit is the MILLIMETRE and the CRS states a
METRE map unit, so the reader has to resolve two units to agree, and the
stated Scale is 0.000999998: as for IfcMapConversion, Scale carries project
units to map units, so it folds the millimetre-to-metre ratio into the
example's grid scale.

- `epset_project_ifc2x3.ifc`: the two sets on IfcProject. The IfcSite also
  carries a complete pair, spelled `ePset_` with other numbers: the project
  pair wins, so the site pair must not be read.
- `map_conversion_ifc4.ifc`: the same model in IFC4, with IfcMapConversion
  from the model context onto an IfcProjectedCRS whose MapUnit is METRE.
  The two must resolve to the same parameters.
- `epset_sites_ifc2x3.ifc`: no project sets; two IfcSites, each with a
  complete `EPset_` pair (the legacy placement). Each yields one operation.
  The first site's Eastings states its own Unit, the METRE map unit.
- `epset_split_pair_ifc2x3.ifc`: ePSet_MapConversion on IfcProject and
  ePSet_ProjectedCRS on IfcSite. A pair is never joined across hosts: refused
  as incomplete.
- `epset_malformed_ifc2x3.ifc`: the project pair with Eastings written as an
  IfcReal instead of the guide's IfcLengthMeasure: refused as invalid.
- `epset_missing_northings_ifc2x3.ifc`: the project pair without Northings:
  refused as missing.
"""
import pathlib
import sys
import uuid

import ifcopenshell
import ifcopenshell.guid

EXAMPLE = {
    "Eastings": 333780.622,
    "Northings": 6246775.891,
    "OrthogonalHeight": 97.457,
    "XAxisAbscissa": 0.990330045,
    "XAxisOrdinate": -0.138731399,
    # The example's grid scale, 0.999998, times the millimetre-to-metre
    # ratio: Scale converts project units to map units (IfcMapConversion).
    "Scale": 0.000999998,
}

CRS = {
    "Name": ("IfcLabel", "EPSG:7856"),
    "Description": ("IfcText", "GDA2020 / MGA zone 56"),
    "GeodeticDatum": ("IfcIdentifier", "GDA2020"),
    "VerticalDatum": ("IfcIdentifier", "AHD"),
    "MapProjection": ("IfcIdentifier", "MGA"),
    "MapZone": ("IfcIdentifier", "56"),
}


def guid(label):
    """A GUID that is stable across regenerations."""
    return ifcopenshell.guid.compress(
        uuid.uuid5(uuid.NAMESPACE_URL, "openbimrs/ifc#451/georef/" + label).hex)


class Model:
    """The shared skeleton: owner history, units, context, project, site."""

    def __init__(self, schema, label):
        self.label = label
        f = self.f = ifcopenshell.file(schema=schema)
        person = f.create_entity("IfcPerson", FamilyName="Generator")
        org = f.create_entity("IfcOrganization", Name="openbim.rs")
        app = f.create_entity(
            "IfcApplication", ApplicationDeveloper=org, Version="1",
            ApplicationFullName="gen_georef_fixtures", ApplicationIdentifier="ggf")
        self.owner = f.create_entity(
            "IfcOwnerHistory",
            OwningUser=f.create_entity(
                "IfcPersonAndOrganization", ThePerson=person, TheOrganization=org),
            OwningApplication=app, ChangeAction="NOCHANGE", CreationDate=0)
        self.millimetre = f.create_entity(
            "IfcSIUnit", UnitType="LENGTHUNIT", Prefix="MILLI", Name="METRE")
        self.metre = f.create_entity("IfcSIUnit", UnitType="LENGTHUNIT", Name="METRE")
        radian = f.create_entity("IfcSIUnit", UnitType="PLANEANGLEUNIT", Name="RADIAN")
        units = f.create_entity("IfcUnitAssignment", Units=[self.millimetre, radian])
        origin = f.create_entity(
            "IfcAxis2Placement3D",
            Location=f.create_entity("IfcCartesianPoint", Coordinates=(0.0, 0.0, 0.0)))
        self.context = f.create_entity(
            "IfcGeometricRepresentationContext", ContextType="Model",
            CoordinateSpaceDimension=3, Precision=1e-5, WorldCoordinateSystem=origin)
        self.project = f.create_entity(
            "IfcProject", GlobalId=guid(label + "/project"), OwnerHistory=self.owner,
            Name="Project", RepresentationContexts=[self.context], UnitsInContext=units)
        self.sites = []

    def site(self, name):
        f = self.f
        site = f.create_entity(
            "IfcSite", GlobalId=guid(self.label + "/" + name), OwnerHistory=self.owner,
            Name=name, CompositionType="ELEMENT",
            ObjectPlacement=f.create_entity(
                "IfcLocalPlacement",
                RelativePlacement=f.create_entity(
                    "IfcAxis2Placement3D",
                    Location=f.create_entity(
                        "IfcCartesianPoint", Coordinates=(0.0, 0.0, 0.0)))))
        self.sites.append(site)
        return site

    def finish(self):
        if self.sites:
            self.f.create_entity(
                "IfcRelAggregates", GlobalId=guid(self.label + "/aggregates"),
                OwnerHistory=self.owner, RelatingObject=self.project,
                RelatedObjects=self.sites)
        return self.f

    def pset(self, host, name, properties):
        """Attach an IfcPropertySet of single values to `host`."""
        f = self.f
        props = []
        for prop_name, (type_name, value, unit) in properties.items():
            props.append(f.create_entity(
                "IfcPropertySingleValue", Name=prop_name,
                NominalValue=f.create_entity(type_name, value), Unit=unit))
        pset = f.create_entity(
            "IfcPropertySet", GlobalId=guid(f"{self.label}/{host.Name}/{name}"),
            OwnerHistory=self.owner, Name=name, HasProperties=props)
        f.create_entity(
            "IfcRelDefinesByProperties",
            GlobalId=guid(f"{self.label}/{host.Name}/{name}/rel"),
            OwnerHistory=self.owner, RelatedObjects=[host],
            RelatingPropertyDefinition=pset)
        return pset


def conversion_properties(values=EXAMPLE, skip=(), overrides=None):
    props = {}
    for name, value in values.items():
        if name in skip:
            continue
        kind = "IfcLengthMeasure" if name in (
            "Eastings", "Northings", "OrthogonalHeight") else "IfcReal"
        props[name] = (kind, value, None)
    props.update(overrides or {})
    return props


def crs_properties(map_unit="METRE"):
    props = {name: (kind, value, None) for name, (kind, value) in CRS.items()}
    if map_unit is not None:
        props["MapUnit"] = ("IfcLabel", map_unit, None)
    return props


def project_pair(model, prefix="ePSet_", **conversion):
    model.pset(model.project, prefix + "MapConversion", conversion_properties(**conversion))
    model.pset(model.project, prefix + "ProjectedCRS", crs_properties())


def epset_project():
    m = Model("IFC2X3", "project")
    project_pair(m)
    site = m.site("Site")
    m.pset(site, "ePset_MapConversion", conversion_properties(
        {**EXAMPLE, "Eastings": 1.0, "Northings": 2.0}))
    m.pset(site, "ePset_ProjectedCRS", crs_properties())
    return m.finish()


def map_conversion_ifc4():
    m = Model("IFC4", "ifc4")
    f = m.f
    crs = f.create_entity(
        "IfcProjectedCRS", **{name: value for name, (_, value) in CRS.items()},
        MapUnit=m.metre)
    f.create_entity(
        "IfcMapConversion", SourceCRS=m.context, TargetCRS=crs, **EXAMPLE)
    m.site("Site")
    return m.finish()


def epset_sites():
    m = Model("IFC2X3", "sites")
    first = m.site("North")
    m.pset(first, "EPset_MapConversion", conversion_properties(
        overrides={"Eastings": ("IfcLengthMeasure", EXAMPLE["Eastings"], m.metre)}))
    m.pset(first, "EPset_ProjectedCRS", crs_properties())
    second = m.site("South")
    m.pset(second, "EPset_MapConversion", conversion_properties(
        {**EXAMPLE, "Eastings": 334000.0, "Northings": 6246000.0}))
    m.pset(second, "EPset_ProjectedCRS", crs_properties())
    return m.finish()


def epset_split_pair():
    m = Model("IFC2X3", "split")
    m.pset(m.project, "ePSet_MapConversion", conversion_properties())
    site = m.site("Site")
    m.pset(site, "ePSet_ProjectedCRS", crs_properties())
    return m.finish()


def epset_malformed():
    m = Model("IFC2X3", "malformed")
    project_pair(m, overrides={"Eastings": ("IfcReal", EXAMPLE["Eastings"], None)})
    return m.finish()


def epset_missing_northings():
    m = Model("IFC2X3", "missing")
    project_pair(m, skip=("Northings",))
    return m.finish()


FIXTURES = {
    "epset_project_ifc2x3.ifc": epset_project,
    "map_conversion_ifc4.ifc": map_conversion_ifc4,
    "epset_sites_ifc2x3.ifc": epset_sites,
    "epset_split_pair_ifc2x3.ifc": epset_split_pair,
    "epset_malformed_ifc2x3.ifc": epset_malformed,
    "epset_missing_northings_ifc2x3.ifc": epset_missing_northings,
}


def main():
    if len(sys.argv) != 2:
        print("usage: gen_georef_fixtures.py <outdir>", file=sys.stderr)
        return 2
    out = pathlib.Path(sys.argv[1])
    out.mkdir(parents=True, exist_ok=True)
    for name, build in FIXTURES.items():
        model = build()
        model.header.file_name.name = name
        model.header.file_name.time_stamp = "1970-01-01T00:00:00"
        target = out / name
        model.write(str(target))
        print(f"{target.stat().st_size:>7} {name}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
