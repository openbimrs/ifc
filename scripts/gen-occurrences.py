"""Generate ifc-occurrence/src/table.rs from the IFC4X3 ADD2 schema."""

import json
from pathlib import Path

T = json.load(open("/tmp/occ.json"))
OUT = Path("ifc-occurrence/src/table")
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
HDR.append("//! rather than left to the caller.")
HDR.append("")
HDR.append("/// One occurrence class: its slots, enum, and permitted type.")
HDR.append("#[derive(Debug, Clone, Copy, PartialEq, Eq)]")
HDR.append("pub struct Occurrence {")
HDR.append("    /// STEP type name, upper-case as stored.")
HDR.append("    pub type_name: &'static str,")
HDR.append("    /// Total attribute count, including inherited.")
HDR.append("    pub arity: usize,")
HDR.append("    /// Slot holding `PredefinedType`, or `None` when the class has none.")
HDR.append("    pub predefined_slot: Option<usize>,")
HDR.append("    /// Permitted `PredefinedType` tokens; empty when there is no enum.")
HDR.append("    pub members: &'static [&'static str],")
HDR.append("    /// The one type class `CorrectTypeAssigned` permits, if any.")
HDR.append("    pub type_class: Option<&'static str>,")
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
    out.append("};")
    out.append("")
    return out

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
