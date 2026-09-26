#!/usr/bin/env python3
"""Report which concrete IFC entities a test run actually authored.

Reads the per-process files written by the `authored-dump` feature in
ifc-model and compares them against the concrete entities declared by
the IFC4X3 schema.

Usage:
    AUTHORED_DUMP=/tmp/dump \
      cargo test --workspace --all-features --features ifc-model/authored-dump
    python3 scripts/authored-coverage.py /tmp/dump          # human report
    python3 scripts/authored-coverage.py /tmp/dump --json   # for the docs

`scripts/gate.sh` runs this with `--json` after its test run and compares
the result with `docs/.vitepress/data/authored-coverage.json`, which the
coverage page is generated from.

Each dump line is `origin<TAB>TYPENAME`.

Only `create` counts as authored -- it is the writer path. `insert` is
the chokepoint every entity lands through, including codec loads and
test fixtures, so it answers "does this type appear at all" rather than
"can a writer produce one". `retype` renames an entity in place and can
name a type no writer ever built.
"""

from __future__ import annotations

import json
import pathlib
import re
import sys

SCHEMA = (
    pathlib.Path(__file__).resolve().parent.parent
    / "references/ifc-spec/ifc4x3-add2/IFC4X3_ADD2.exp"
)


def concrete_entities(schema: pathlib.Path) -> set[str]:
    """Every entity the schema declares that is not an ABSTRACT SUPERTYPE."""
    text = schema.read_text(errors="ignore")
    found = set()
    for match in re.finditer(r"^ENTITY\s+(\w+)([^;]*);", text, re.M):
        if "ABSTRACT SUPERTYPE" not in match.group(2):
            found.add(match.group(1).upper())
    return found


def read_dump(directory: pathlib.Path) -> dict[str, set[str]]:
    """Union every per-process file, keyed by origin."""
    by_origin: dict[str, set[str]] = {}
    for path in sorted(directory.glob("*.txt")):
        for line in path.read_text(errors="ignore").splitlines():
            line = line.strip()
            if not line or "\t" not in line:
                continue
            # origin FIRST, then the name. Reading these the other way
            # round yields a confident zero rather than an error.
            origin, name = line.split("\t", 1)
            by_origin.setdefault(origin, set()).add(name.strip().upper())
    return by_origin


def main() -> int:
    args = sys.argv[1:]
    as_json = "--json" in args
    args = [arg for arg in args if arg != "--json"]
    if len(args) != 1:
        print(__doc__, file=sys.stderr)
        return 2
    directory = pathlib.Path(args[0])
    if not directory.is_dir():
        print(f"not a directory: {directory}", file=sys.stderr)
        return 2
    if not SCHEMA.is_file():
        print(f"schema not found: {SCHEMA}", file=sys.stderr)
        return 2

    by_origin = read_dump(directory)
    if not by_origin:
        print(f"no dump files in {directory}", file=sys.stderr)
        print("was AUTHORED_DUMP set, with --features ifc-model/authored-dump?", file=sys.stderr)
        return 1

    concrete = concrete_entities(SCHEMA)
    created = by_origin.get("create", set())
    inserted = by_origin.get("insert", set())
    retyped = by_origin.get("retype", set())
    proven = concrete & created
    gap = sorted(concrete - created)
    # Seen landing in a model, but never through a writer. These are the
    # types a fixture or a codec produced and no authoring path can.
    seen_only = sorted(concrete & (inserted | retyped) - created)

    if as_json:
        report = {
            "schema": SCHEMA.name,
            "concrete": len(concrete),
            "authored": len(proven),
            "landed": len(concrete & inserted),
            "renamed": len(concrete & retyped),
            "seen_but_unwritable": seen_only,
            "unproven": gap,
        }
        print(json.dumps(report, indent=2))
        return 0

    print(f"concrete entities  : {len(concrete)}")
    print(f"authored (create)  : {len(proven)}  ({100 * len(proven) / len(concrete):.1f}%)")
    print(f"landed   (insert)  : {len(concrete & inserted)}")
    print(f"renamed  (retype)  : {len(concrete & retyped)}")
    print(f"seen but unwritable: {len(seen_only)}")
    print(f"unproven           : {len(gap)}")
    if gap:
        print()
        for name in gap:
            print(f"    {name}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
