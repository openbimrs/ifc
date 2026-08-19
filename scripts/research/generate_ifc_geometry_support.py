#!/usr/bin/env python3
"""Build the reviewed IFC geometry declaration ownership ledger.

This is a maintenance tool, never a Cargo build dependency. It joins the
normative declaration manifest with the current bridge module and a conservative
format-neutral capability family. Review diffs; heuristics assign ownership but
do not prove implementation.
"""

from __future__ import annotations

import csv
import re
import sys
from pathlib import Path

NATIVE_FUNCTIONS = {
    name.lower()
    for name in (
        "IfcCrossProduct",
        "IfcDotProduct",
        "IfcNormalise",
        "IfcScalarTimesVector",
        "IfcVectorDifference",
        "IfcVectorSum",
    )
}


def rust_owner(source_root: Path, name: str) -> str:
    candidates = []
    needle = re.compile(rf"\b{re.escape(name)}\b", re.IGNORECASE)
    normalized = name.removeprefix("Ifc").lower()
    for path in source_root.rglob("*.rs"):
        if path.name in {"lib.rs", "error.rs", "functions.rs", "declarations.rs"}:
            continue
        text = path.read_text(encoding="utf-8")
        matches = len(needle.findall(text))
        if matches:
            relative = path.relative_to(source_root).with_suffix("")
            if relative.name == "mod":
                relative = relative.parent
            module = "::".join(relative.parts)
            stem = path.stem.replace("_", "").lower()
            rank = (
                0 if stem and stem in normalized else 1,
                -matches,
                len(relative.parts),
                module,
            )
            candidates.append((rank, module))
    if not candidates:
        raise RuntimeError(f"no bridge owner found for {name}")
    return min(candidates)[1]


def neutral_owner(name: str, resource: str, kind: str) -> str:
    lower = name.lower()
    if kind == "function":
        return "geom-core" if name.lower() in NATIVE_FUNCTIONS else "ifc-geometry"
    if resource.lower() == "ifcgeometricconstraintresource":
        return "ifc-geometry::constraint + geom-model::NodeId"
    if any(word in lower for word in ("tessellated", "triangulated", "polygonalfaceset")):
        return "geom-mesh"
    if any(word in lower for word in ("solid", "boolean", "csg", "halfspace", "extruded", "revolved", "sectioned")):
        return "geom-model::SolidOperation"
    topology_prefixes = (
        "ifcvertex",
        "ifcedge",
        "ifcorientededge",
        "ifcsubedge",
        "ifcloop",
        "ifcpath",
        "ifcface",
        "ifcshell",
        "ifcadvancedface",
    )
    if lower.startswith(topology_prefixes) or any(
        word in lower for word in ("brep", "topological", "connectedfaceset", "facebound")
    ):
        return "geom-topology"
    if any(word in lower for word in ("surface", "plane", "cylindrical", "spherical", "toroidal")):
        return "geom-surface + geom-model::SurfaceRelation"
    if any(word in lower for word in ("curve", "line", "polyline", "conic", "circle", "ellipse", "clothoid")):
        return "geom-curve + geom-model::CurveRelation"
    if any(word in lower for word in ("point", "vector", "direction", "axis", "placement", "transformation", "coordinate")):
        return "geom-core"
    return "geom-model"


def main() -> None:
    if len(sys.argv) != 4:
        raise SystemExit("usage: generate_ifc_geometry_support.py DECLARATIONS_TSV SRC_DIR OUTPUT_TSV")
    manifest, source_root, output = map(Path, sys.argv[1:])
    rows = list(csv.DictReader(manifest.open(encoding="utf-8"), delimiter="\t"))
    output.parent.mkdir(parents=True, exist_ok=True)
    fields = [
        "resource",
        "kind",
        "name",
        "abstract",
        "type_kind",
        "bridge_owner",
        "neutral_owner",
        "status",
    ]
    with output.open("w", encoding="utf-8", newline="") as handle:
        writer = csv.DictWriter(handle, fields, delimiter="\t", lineterminator="\n")
        writer.writeheader()
        for row in rows:
            kind = row["kind"]
            name = row["name"]
            if kind == "function":
                owner = "resource::functions"
                status = "native-primitive" if name.lower() in NATIVE_FUNCTIONS else "scaffolded"
            else:
                owner = rust_owner(source_root, name)
                if kind == "entity":
                    status = "inventory" if row["abstract"] == "true" else "view-or-family"
                else:
                    status = "modeled-type"
            writer.writerow({
                **row,
                "bridge_owner": owner,
                "neutral_owner": neutral_owner(name, row["resource"], kind),
                "status": status,
            })
    print(f"wrote {len(rows)} declarations to {output}")


if __name__ == "__main__":
    main()
