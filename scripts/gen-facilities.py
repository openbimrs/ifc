"""Generate ifc-spatial/src/facility/table.rs from the IFC4X3 ADD2 schema.

    python3 scripts/gen-facilities.py

Needs only `references/ifc-spec/ifc4x3-add2/IFC4X3_ADD2.exp` (see
`scripts/fetch-ifc-schemas.sh`) and `rustfmt`. The rows come from
`scripts/ifc4x3_catalogue.py`, which also records the one facility
(`IfcBuilding`) `ifc-spatial` authors through its container path and this
table leaves out. The emitted file is then formatted with
`rustfmt --edition 2021`, so a regenerate on a clean checkout reproduces
the committed file byte for byte (`git diff --exit-code` proves it).
"""

import subprocess

from ifc4x3_catalogue import ROOT, Schema, facilities

CAT = facilities(Schema())
OUT = ROOT / "ifc-spatial/src/facility/table.rs"
OUT.parent.mkdir(parents=True, exist_ok=True)

o = []
o.append("//! Generated facility catalogue. Do not edit by hand.")
o.append("//!")
o.append("//! Source: `references/ifc-spec/ifc4x3-add2/IFC4X3_ADD2.exp`")
o.append("//! Regenerate: `python3 scripts/gen-facilities.py`")
o.append("")
o.append("use super::Facility;")
o.append("")
for r in CAT:
    konst = r["upper"]
    o.append("/// `%s`." % r["entity"])
    o.append("pub const %s: Facility = Facility {" % konst)
    o.append("    type_name: \"%s\"," % r["upper"])
    o.append("    arity: %d," % r["arity"])
    if r["pt"] >= 0:
        o.append("    predefined_slot: Some(%d)," % r["pt"])
    else:
        o.append("    predefined_slot: None,")
    if r["ut"] >= 0:
        o.append("    usage_slot: Some(%d)," % r["ut"])
    else:
        o.append("    usage_slot: None,")
    if r["tokens"]:
        o.append("    members: &[")
        for t in r["tokens"]:
            o.append("        \"%s\"," % t)
        o.append("    ],")
    else:
        o.append("    members: &[],")
    o.append("};")
    o.append("")
o.append("/// Every facility class this crate can author.")
o.append("pub const ALL: &[Facility] = &[")
for r in CAT:
    o.append("    %s," % r["upper"])
o.append("];")

OUT.write_text("\n".join(o) + "\n")
print("wrote", OUT, len(o), "lines,", len(CAT), "facilities")

subprocess.run(["rustfmt", "--edition", "2021", str(OUT)], check=True)
print("formatted with rustfmt")
