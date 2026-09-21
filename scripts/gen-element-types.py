"""Generate ifc-element-type/src/table.rs from the IFC4X3 ADD2 schema."""

import json
from pathlib import Path

T = json.loads(Path("/tmp/typefinal.json").read_text())
OUT = Path("ifc-element-type/src/table.rs")

HDR = []
HDR.append("//! The generated element-type catalogue.")
HDR.append("//!")
HDR.append("//! Source: `references/ifc-spec/ifc4x3-add2/IFC4X3_ADD2.exp`, via")
HDR.append("//! `scripts/gen-element-types.py`. Do not edit by hand.")
HDR.append("//!")
HDR.append("//! # Two slot layouts, not one")
HDR.append("//!")
HDR.append("//! Element types put `RepresentationMaps` at 6 and `Tag` at 7.")
HDR.append("//! Resource and process types put `Identification` at 6 and")
HDR.append("//! `LongDescription` at 7. Nine types use the second layout, so")
HDR.append("//! a writer that assumed `Tag` would file a tag as a description")
HDR.append("//! on every resource and process type.")
HDR.append("//!")
HDR.append("//! `PredefinedType` lands at 9 for most types, 10 for")
HDR.append("//! `IfcFurnitureType`, and 11 for the six resource types, whose")
HDR.append("//! supertype interposes `BaseCosts` and `BaseQuantity`.")
HDR.append("")
HDR.append("/// Which supertype layout an element type follows.")
HDR.append("#[derive(Debug, Clone, Copy, PartialEq, Eq)]")
HDR.append("pub enum Family {")
HDR.append("    /// `RepresentationMaps` at 6, `Tag` at 7, `ElementType` at 8.")
HDR.append("    Element,")
HDR.append("    /// `Identification` at 6, `LongDescription` at 7,")
HDR.append("    /// `ResourceType` or `ProcessType` at 8.")
HDR.append("    ResourceOrProcess,")
HDR.append("}")
HDR.append("")
HDR.append("/// One element type: its STEP name, slots, and enum tokens.")
HDR.append("#[derive(Debug, Clone, Copy, PartialEq, Eq)]")
HDR.append("pub struct ElementType {")
HDR.append("    /// STEP type name, upper-case as stored.")
HDR.append("    pub type_name: &'static str,")
HDR.append("    /// Total attribute count, including inherited.")
HDR.append("    pub arity: usize,")
HDR.append("    /// Slot holding `PredefinedType`.")
HDR.append("    pub predefined_slot: usize,")
HDR.append("    /// Whether `PredefinedType` is itself optional.")
HDR.append("    pub predefined_optional: bool,")
HDR.append("    /// Name of the attribute `USERDEFINED` falls back to.")
HDR.append("    pub fallback_attr: &'static str,")
HDR.append("    /// Slot of that fallback attribute. Always 8.")
HDR.append("    pub fallback_slot: usize,")
HDR.append("    /// Which supertype layout slots 6 and 7 follow.")
HDR.append("    pub family: Family,")
HDR.append("    /// Permitted `PredefinedType` tokens.")
HDR.append("    pub members: &'static [&'static str],")
HDR.append("}")
HDR.append("")

def row(t):
    fam = "Family::ResourceOrProcess" if t["slot7"] == "LongDescription" else "Family::Element"
    members = ", ".join("\"%s\"" % m for m in t["members"])
    out = []
    out.append("/// `%s`, %d permitted tokens." % (t["entity"], len(t["members"])))
    out.append("pub const %s: ElementType = ElementType {" % t["upper"])
    out.append("    type_name: \"%s\"," % t["upper"])
    out.append("    arity: %d," % t["arity"])
    out.append("    predefined_slot: %d," % t["predefined_slot"])
    out.append("    predefined_optional: %s," % ("true" if t["predefined_optional"] else "false"))
    out.append("    fallback_attr: \"%s\"," % t["fallback_attr"])
    out.append("    fallback_slot: %d," % t["fallback_slot"])
    out.append("    family: %s," % fam)
    out.append("    members: &[%s]," % members)
    out.append("};")
    out.append("")
    return out

# Shard alphabetically so the split is mechanical and stable: a new
# schema version drops a type into an existing shard without moving
# its neighbours.
#
# Size these against the POST-rustfmt line count, not the emitted one.
# rustfmt expands each const to roughly 1.7x what this writes, and the
# monolith gate measures the formatted file. Six shards leaves every
# one near 400 formatted lines, half the 800-line limit, so a future
# schema can grow a shard without tripping it.
SHARDS = [("a_c", "A", "C"), ("d_f", "D", "F"), ("g_k", "G", "K"), ("l_p", "L", "P"), ("q_s", "Q", "S"), ("t_z", "T", "Z")]

DIR = Path("ifc-element-type/src/table")
DIR.mkdir(parents=True, exist_ok=True)

def shard_of(name):
    initial = name[3]
    for mod, lo, hi in SHARDS:
        if lo <= initial <= hi:
            return mod
    raise AssertionError(name)

buckets = {mod: [] for mod, _, _ in SHARDS}
for t in T:
    buckets[shard_of(t["upper"])].extend(row(t))

for mod, lo, hi in SHARDS:
    body = []
    body.append("//! Element types %s-%s. Generated; do not edit." % (lo, hi))
    body.append("")
    body.append("use super::{ElementType, Family};")
    body.append("")
    body.extend(buckets[mod])
    (DIR / ("%s.rs" % mod)).write_text("\n".join(body) + "\n")
    print("  %s.rs %d lines" % (mod, len(body)))

mod_lines = list(HDR)
for mod, _, _ in SHARDS:
    mod_lines.append("mod %s;" % mod)
mod_lines.append("")
for mod, _, _ in SHARDS:
    mod_lines.append("pub use %s::*;" % mod)
mod_lines.append("")
mod_lines.append("/// Every element type in the catalogue.")
mod_lines.append("pub const ALL: &[ElementType] = &[")
for t in T:
    mod_lines.append("    %s," % t["upper"])
mod_lines.append("];")
mod_lines.append("")

(DIR / "mod.rs").write_text("\n".join(mod_lines) + "\n")
print("wrote table/mod.rs %d lines, %d types" % (len(mod_lines), len(T)))

old = Path("ifc-element-type/src/table.rs")
if old.exists():
    old.unlink()
    print("removed flat table.rs")
