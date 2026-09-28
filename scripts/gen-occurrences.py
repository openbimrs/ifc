"""Generate crates/ifc-occurrence/src/table/ from the IFC4X3 ADD2 schema.

    python3 scripts/gen-occurrences.py

Needs `references/ifc-spec/ifc4x3-add2/IFC4X3_ADD2.exp`, and for the
per-release type pairing (#214) `ifc4-add2-tc1/IFC4.exp` and
`ifc2x3-tc1/IFC2X3_TC1.exp` (see `scripts/fetch-ifc-schemas.sh`), and
`rustfmt`. The rows come from `scripts/ifc4x3_catalogue.py`, which also
records the nine distribution classes `ifc-systems` authors and this
catalogue leaves out. The emitted files are then formatted with
`rustfmt --edition 2021`, so a regenerate on a clean checkout reproduces
the committed shards byte for byte (`git diff --exit-code` proves it).
"""

import re
import subprocess

from ifc4x3_catalogue import ROOT, Schema, occurrences

T = occurrences(Schema())

# The type class each earlier release pairs with an occurrence (#214).
#
# IFC4 ADD2 TC1 states it as a WHERE rule, `CorrectTypeAssigned` or, on
# `IfcDoor` and `IfcWindow`, `CorrectStyleAssigned`:
#   ('IFC4.IFCDOORTYPE' IN TYPEOF(SELF\IfcObject.IsTypedBy[1].RelatingType))
# read here from the class or the nearest supertype that states it.
#
# IFC2X3 TC1 states none: `IfcRelDefinesByType.RelatingType` is any
# `IfcTypeObject`, and its `IfcTypeProduct` is
# `SUPERTYPE OF (ONEOF (IfcDoorStyle, IfcElementType, IfcWindowStyle))`.
# Doors and windows take `IfcDoorStyle` and `IfcWindowStyle`, which IFC2X3
# declares in place of `IfcDoorType` and `IfcWindowType`; every other class
# takes the `IfcElementType` subtype the later releases pair with it, where
# IFC2X3 declares that class, and none where it does not.
IFC4 = Schema(ROOT / "references/ifc-spec/ifc4-add2-tc1/IFC4.exp")
IFC2X3 = Schema(ROOT / "references/ifc-spec/ifc2x3-tc1/IFC2X3_TC1.exp")
RULE = re.compile(r"Correct(?:Type|Style)Assigned\s*:.*?IN\s+TYPEOF", re.S)
STYLES = {"IFCDOOR": "IFCDOORSTYLE", "IFCWINDOW": "IFCWINDOWSTYLE"}


def concrete(s, n):
    return n in s.bodies and not s.is_abstract(n)


def ifc4_pairing(n):
    if not concrete(IFC4, n):
        return None
    for e in IFC4.chain(n):
        m = RULE.search(IFC4.bodies.get(e, ""))
        if m:
            return re.search(r"IFC4\.(IFC[A-Z0-9]+)", m.group(0)).group(1)
    return None


def ifc2x3_pairing(n, ifc4x3_class):
    if not concrete(IFC2X3, n):
        return None
    cls = STYLES.get(n, ifc4x3_class)
    return cls if cls and concrete(IFC2X3, cls) else None


for t in T:
    t["ifc4_type_class"] = ifc4_pairing(t["upper"])
    t["ifc2x3_type_class"] = ifc2x3_pairing(t["upper"], t.get("type_class"))
OUT = ROOT / "crates/ifc-occurrence/src/table"
OUT.mkdir(parents=True, exist_ok=True)

HDR = []
HDR.append("//! The generated occurrence catalogue.")
HDR.append("//!")
HDR.append("//! Source: `references/ifc-spec/ifc4x3-add2/IFC4X3_ADD2.exp`, via")
HDR.append("//! `scripts/gen-occurrences.py`. Do not edit by hand.")
HDR.append("//!")
HDR.append("//! # Two rules, and why the second one needs the type catalogue")
HDR.append("//!")
HDR.append("//! `CorrectPredefinedType` is the familiar one: USERDEFINED")
HDR.append("//! without `ObjectType` names nothing.")
HDR.append("//!")
HDR.append("//! `CorrectTypeAssigned` is stronger and has no analogue on the")
HDR.append("//! type side. An occurrence may be typed by at most one type,")
HDR.append("//! and that type must be the one class the schema pairs with it:")
HDR.append("//! an `IfcPump` takes an `IfcPumpType` and nothing else. The")
HDR.append("//! pairing is a fact about the schema, so it is recorded here")
HDR.append("//! rather than left to the caller, once per release (#214):")
HDR.append("//! IFC4 pairs an `IfcDoor` with `IfcDoorType` as IFC4X3 does, while")
HDR.append("//! IFC2X3, which declares no such rule and no `IfcDoorType`, types")
HDR.append("//! it by an `IfcDoorStyle`. The IFC4 and IFC2X3 columns come from")
HDR.append("//! `references/ifc-spec/ifc4-add2-tc1/IFC4.exp` and")
HDR.append("//! `references/ifc-spec/ifc2x3-tc1/IFC2X3_TC1.exp`.")
HDR.append("")
HDR.append("/// One occurrence class: its slots, enum, and permitted type.")
HDR.append("#[derive(Debug, Clone, Copy, PartialEq, Eq)]")
HDR.append("#[non_exhaustive]")
HDR.append("pub struct Occurrence {")
HDR.append("    /// STEP type name, upper-case as stored.")
HDR.append("    pub type_name: &'static str,")
HDR.append("    /// Total attribute count, including inherited.")
HDR.append("    pub arity: usize,")
HDR.append("    /// Slot holding `PredefinedType`, or `None` when the class has none.")
HDR.append("    pub predefined_slot: Option<usize>,")
HDR.append("    /// Permitted `PredefinedType` tokens; empty when there is no enum.")
HDR.append("    pub members: &'static [&'static str],")
HDR.append("    /// The one type class IFC4X3 ADD2's `CorrectTypeAssigned` permits,")
HDR.append("    /// if any.")
HDR.append("    pub type_class: Option<&'static str>,")
HDR.append("    /// The one type class IFC4 ADD2 TC1's `CorrectTypeAssigned` (or,")
HDR.append("    /// on doors and windows, `CorrectStyleAssigned`) permits, if IFC4")
HDR.append("    /// declares the class and pairs one.")
HDR.append("    pub ifc4_type_class: Option<&'static str>,")
HDR.append("    /// The type class IFC2X3 TC1 pairs with the class: `IfcDoorStyle`")
HDR.append("    /// and `IfcWindowStyle` for doors and windows, otherwise")
HDR.append("    /// [`Self::type_class`] where IFC2X3 declares it, else none.")
HDR.append("    pub ifc2x3_type_class: Option<&'static str>,")
HDR.append("}")
HDR.append("")

def row(t):
    out = []
    out.append("/// `%s`, %d permitted tokens." % (t["entity"], len(t["members"])))
    out.append("pub const %s: Occurrence = Occurrence {" % t["upper"])
    out.append("    type_name: \"%s\"," % t["upper"])
    out.append("    arity: %d," % t["arity"])
    ps = t["predef_slot"]
    out.append("    predefined_slot: %s," % ("Some(%d)" % ps if ps >= 0 else "None"))
    mem = ", ".join("\"%s\"" % m for m in t["members"])
    out.append("    members: &[%s]," % mem)
    tc = t.get("type_class") or ""
    out.append("    type_class: %s," % ("Some(\"%s\")" % tc if tc else "None"))
    for key in ("ifc4_type_class", "ifc2x3_type_class"):
        tc = t.get(key) or ""
        out.append("    %s: %s," % (key, "Some(\"%s\")" % tc if tc else "None"))
    out.append("};")
    out.append("")
    return out

# Shard by type count, not by letter range. `rustfmt` expands the emitted
# literals to roughly 1.7x their line count, and no source file may exceed
# 800 lines (`crates/ifc-model/tests/no_monolithic_files.rs`), so an even
# split by count is what keeps every shard under the gate.
N = 5
per = (len(T) + N - 1) // N
chunks = [T[i * per:(i + 1) * per] for i in range(N)]
chunks = [c for c in chunks if c]
names = ["part%d" % (i + 1) for i in range(len(chunks))]

for mod, chunk in zip(names, chunks):
    body = ["//! Generated occurrence rows. Do not edit by hand.", ""]
    body.append("use super::Occurrence;")
    body.append("")
    for t in chunk:
        body.extend(row(t))
    (OUT / ("%s.rs" % mod)).write_text("\n".join(body) + "\n")
    print(" ", mod, len(chunk), "types")

mod_lines = list(HDR)
for mod in names:
    mod_lines.append("mod %s;" % mod)
mod_lines.append("")
for mod in names:
    mod_lines.append("pub use %s::*;" % mod)
mod_lines.append("")
mod_lines.append("/// Every occurrence class in the catalogue.")
mod_lines.append("pub const ALL: &[Occurrence] = &[")
for t in T:
    mod_lines.append("    %s," % t["upper"])
mod_lines.append("];")
mod_lines.append("")

(OUT / "mod.rs").write_text("\n".join(mod_lines) + "\n")
print("wrote table/mod.rs", len(mod_lines), "lines,", len(T), "occurrences")

subprocess.run(
    ["rustfmt", "--edition", "2021"] + sorted(str(p) for p in OUT.glob("*.rs")),
    check=True,
)
print("formatted with rustfmt")
