"""Sync the TSV support ledger's function rows to the verified Rust registry.

Truth source is ifc-geometry/src/resource/functions.rs, whose Implemented
rows are independently verified by the declaration_manifest test
`implemented_functions_are_named_by_their_owner_module`.
"""

import re
from pathlib import Path

reg = Path("ifc-geometry/src/resource/functions.rs").read_text()

status_word = {
    "IMPLEMENTED": "implemented",
    "NATIVE": "native-primitive",
    "NOT_APPLICABLE": "not-applicable",
    "SCAFFOLDED": "scaffolded",
}

entries = re.findall(
    r'name:\s*"([^"]+)",\s*owner:\s*"([^"]+)",\s*status:\s*(\w+)',
    reg,
)
assert len(entries) == 28, f"expected 28 registry rows, found {len(entries)}"

truth = {name.lower(): status_word[st] for name, _owner, st in entries}

tsv_path = Path("ifc-geometry/data/ifc4-add2-tc1-geometry-support.tsv")
lines = tsv_path.read_text().splitlines()
out = [lines[0]]
changed = 0
for line in lines[1:]:
    f = line.split("\t")
    if f[1] == "function":
        want = truth[f[2]]
        if f[7] != want:
            f[7] = want
            changed += 1
    out.append("\t".join(f))

tsv_path.write_text("\n".join(out) + "\n")
print("rows updated:", changed)

counts = {}
for line in out[1:]:
    f = line.split("\t")
    if f[1] == "function":
        counts[f[7]] = counts.get(f[7], 0) + 1
print("function status counts:", counts)
