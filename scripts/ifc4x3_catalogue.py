"""Derive the element-type, occurrence and facility catalogues from IFC4X3 ADD2.

This is the input step of `scripts/gen-element-types.py`,
`scripts/gen-occurrences.py` and `scripts/gen-facilities.py`. Each
generator imports it; none reads an intermediate file. Run from anywhere:

    python3 scripts/ifc4x3_catalogue.py types > types.json
    python3 scripts/ifc4x3_catalogue.py occurrences > occurrences.json
    python3 scripts/ifc4x3_catalogue.py facilities > facilities.json

to inspect the rows a generator will emit.

The only input is the local, unredistributable schema checkout at
`references/ifc-spec/ifc4x3-add2/IFC4X3_ADD2.exp` (fetch it with
`scripts/fetch-ifc-schemas.sh`). The walk is a deliberately small
regex reader over that one file, not a general EXPRESS parser: it is
enough for entity bodies, `SUBTYPE OF`, explicit attributes and
enumerations, and the two WHERE rules the catalogues need.
"""

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SCHEMA = ROOT / "references/ifc-spec/ifc4x3-add2/IFC4X3_ADD2.exp"

# Concrete `IfcElement` subtypes whose authoring another crate owns, so
# `ifc-occurrence` leaves them out. They are exactly the variants of
# `ifc-systems::authoring::distribution::DistributionElementKind`: the
# generic distribution supertypes the flow reader traverses. `occurrences`
# checks every name still appears in that file, so a change there cannot
# silently drift from this list.
SYSTEMS_OWNED = (
    "IFCDISTRIBUTIONELEMENT",
    "IFCENERGYCONVERSIONDEVICE",
    "IFCFLOWCONTROLLER",
    "IFCFLOWFITTING",
    "IFCFLOWMOVINGDEVICE",
    "IFCFLOWSEGMENT",
    "IFCFLOWSTORAGEDEVICE",
    "IFCFLOWTERMINAL",
    "IFCFLOWTREATMENTDEVICE",
)
SYSTEMS_OWNER = ROOT / "ifc-systems/src/authoring/distribution.rs"

# Concrete facilities `ifc-spatial` authors through its container path
# (`SpatialKind`), not through the generated facility table. `facilities`
# checks every name still appears in that file.
SPATIAL_OWNED = ("IFCBUILDING",)
SPATIAL_OWNER = ROOT / "ifc-spatial/src/authoring/mod.rs"


class Schema:
    """Entity bodies, casing, and enumerations of one EXPRESS file."""

    def __init__(self, path=SCHEMA):
        if not path.is_file():
            sys.exit("missing %s; run scripts/fetch-ifc-schemas.sh" % path)
        exp = path.read_text(errors="ignore")
        pat = re.compile(r"^ENTITY\s+(\w+)(.*?)END_ENTITY;", re.S | re.M | re.I)
        self.bodies = {m.group(1).upper(): m.group(2) for m in pat.finditer(exp)}
        self.cased = {m.group(1).upper(): m.group(1) for m in pat.finditer(exp)}
        self.enums = {}
        for m in re.finditer(
            r"^TYPE\s+(\w+)\s*=\s*ENUMERATION\s+OF\s*\((.*?)\);", exp, re.S | re.M
        ):
            self.enums[m.group(1).upper()] = [
                x.strip() for x in m.group(2).split(",") if x.strip()
            ]

    def sup(self, n):
        m = re.search(r"SUBTYPE\s+OF\s*\(\s*(\w+)", self.bodies.get(n.upper(), ""), re.I)
        return m.group(1).upper() if m else None

    def chain(self, n):
        """`n` followed by its supertypes, nearest first."""
        out, cur = [], n.upper()
        while cur:
            out.append(cur)
            cur = self.sup(cur)
        return out

    def attrs(self, n):
        """Explicit attributes declared on `n` itself, as (name, type)."""
        b = self.bodies.get(n.upper(), "")
        b = re.sub(r"^.*?;", "", b, count=1, flags=re.S)
        b = re.split(r"^\s*(?:INVERSE|WHERE|DERIVE|UNIQUE)\b", b, flags=re.I | re.M)[0]
        out = []
        for line in b.split(";"):
            line = " ".join(line.split())
            if not line or ":" not in line:
                continue
            nm, ty = line.split(":", 1)
            nm = nm.strip()
            if re.fullmatch(r"\w+", nm):
                out.append((nm, ty.strip()))
        return out

    def slots(self, n):
        """All explicit attributes in STEP slot order, inherited first."""
        out = []
        for e in reversed(self.chain(n)):
            out.extend(self.attrs(e))
        return out

    def is_abstract(self, n):
        return "ABSTRACT" in self.bodies[n].split(";")[0]


def element_types(s):
    """Concrete `IfcTypeObject` subtypes that declare `PredefinedType`.

    Ordered by the entity's schema spelling. The `USERDEFINED` fallback
    attribute is the last `EXISTS(...)` in `CorrectPredefinedType`.
    """
    rows = []
    for up, body in s.bodies.items():
        if "IFCTYPEOBJECT" not in s.chain(up)[1:]:
            continue
        if re.search(r"ABSTRACT\s+SUPERTYPE", body, re.I):
            continue
        ss = s.slots(up)
        names = [a for a, _ in ss]
        if "PredefinedType" not in names:
            continue
        i = names.index("PredefinedType")
        ty = ss[i][1]
        enum = re.sub(r"^OPTIONAL\s+", "", ty, flags=re.I).strip()
        rule = re.search(r"CorrectPredefinedType\s*:(.*?);", " ".join(body.split()), re.S)
        hits = re.findall(r"EXISTS\s*\(\s*(?:SELF\\)?([\w.]+)\s*\)", rule.group(1))
        attr = hits[-1].split(".")[-1]
        if attr not in names:
            sys.exit("fallback attribute %s not found on %s" % (attr, up))
        rows.append({
            "entity": s.cased[up],
            "upper": up,
            "arity": len(ss),
            "predefined_slot": i,
            "predefined_optional": ty.upper().startswith("OPTIONAL"),
            "fallback_attr": attr,
            "fallback_slot": names.index(attr),
            "slot7": names[7] if len(names) > 7 else "",
            "slot6": names[6] if len(names) > 6 else "",
            "enum": enum,
            "members": s.enums.get(enum.upper(), []),
        })
    rows.sort(key=lambda r: r["entity"])
    return rows


def occurrences(s):
    """Concrete `IfcElement` subtypes, minus `SYSTEMS_OWNED`, by upper name.

    `type_class` is the one class `CorrectTypeAssigned` permits, if any.
    """
    owner = SYSTEMS_OWNER.read_text()
    for n in SYSTEMS_OWNED:
        if '"%s"' % n not in owner:
            sys.exit("%s no longer names %s; update SYSTEMS_OWNED" % (SYSTEMS_OWNER, n))
    ta = re.compile(r"CorrectTypeAssigned.*?IN TYPEOF", re.S)
    cls = re.compile(r"IFC4X3_ADD2\.(IFC[A-Z0-9]+)")
    rows = []
    for n in sorted(s.bodies):
        if n in SYSTEMS_OWNED or s.is_abstract(n) or n.endswith("TYPE"):
            continue
        ch = s.chain(n)
        if "IFCELEMENT" not in ch:
            continue
        ss = s.slots(n)
        names = [a for a, _ in ss]
        pd = names.index("PredefinedType") if "PredefinedType" in names else -1
        enum = ""
        if pd >= 0:
            m = re.search(r"(Ifc\w+Enum)", ss[pd][1], re.I)
            enum = m.group(1).upper() if m else ""
        m = ta.search(s.bodies[n])
        rows.append({
            "entity": s.cased[n],
            "upper": n,
            "arity": len(ss),
            "predef_slot": pd,
            "members": s.enums.get(enum, []),
            "type_class": cls.search(m.group(0)).group(1) if m else None,
        })
    return rows


def facilities(s):
    """Concrete `IfcFacility`/`IfcFacilityPart` subtypes, minus `SPATIAL_OWNED`.

    Ordered by upper-case name. `pt` and `ut` are the absolute slots of
    `PredefinedType` and `UsageType`, or -1 when the class declares none.
    """
    owner = SPATIAL_OWNER.read_text()
    for n in SPATIAL_OWNED:
        if '"%s"' % n not in owner:
            sys.exit("%s no longer names %s; update SPATIAL_OWNED" % (SPATIAL_OWNER, n))
    rows = []
    for n in sorted(s.bodies):
        if n in SPATIAL_OWNED or s.is_abstract(n):
            continue
        ch = s.chain(n)
        if "IFCFACILITY" not in ch and "IFCFACILITYPART" not in ch:
            continue
        ss = s.slots(n)
        names = [a for a, _ in ss]
        pt = names.index("PredefinedType") if "PredefinedType" in names else -1
        ut = names.index("UsageType") if "UsageType" in names else -1
        enum = re.sub(r"^OPTIONAL\s+", "", ss[pt][1], flags=re.I).strip() if pt >= 0 else None
        rows.append({
            "entity": s.cased[n],
            "upper": n,
            "arity": len(ss),
            "pt": pt,
            "ut": ut,
            "enum": enum,
            "tokens": s.enums.get(enum.upper(), []) if enum else [],
            "part": "IFCFACILITYPART" in ch,
        })
    return rows


if __name__ == "__main__":
    kinds = {"types": element_types, "occurrences": occurrences, "facilities": facilities}
    if len(sys.argv) != 2 or sys.argv[1] not in kinds:
        sys.exit("usage: ifc4x3_catalogue.py {types|occurrences|facilities}")
    json.dump(kinds[sys.argv[1]](Schema()), sys.stdout, indent=1)
    print()
