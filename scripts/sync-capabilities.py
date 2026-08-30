#!/usr/bin/env python3
"""Derive the published capability tables from the source that implements them.

A hand-maintained capability matrix drifts, and a matrix that drifts is worse
than none: it is read as a promise. ADR 0005 recorded this as the main
documentation risk and asked for the matrix to become machine-derived. This
script is that.

The generated tables live between sentinel comments in docs/capabilities.md.
Everything outside the sentinels is prose a human owns.

Run with ``--check`` in CI to fail when the page and the code disagree.
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
TARGET = ROOT / "docs" / "capabilities.md"
DISPATCH = ROOT / "ifc-geometry" / "src" / "lower" / "dispatch.rs"
PROFILE = ROOT / "ifc-geometry" / "src" / "lower" / "profile.rs"
# Correctly-cased entity names, already maintained and asserted against the
# published schema counts. Re-deriving casing from uppercase STEP names would
# need a word-splitting heuristic that fails silently on the next entity.
CASING = ROOT / "ifc-geometry" / "tests" / "schema_coverage.rs"

IMPLEMENTED = '<span class="status-implemented">Implemented</span>'
PLANNED = '<span class="status-partial">Planned</span>'

BEGIN_GEOMETRY = "<!-- CAPABILITIES:GEOMETRY:BEGIN -->"
END_GEOMETRY = "<!-- CAPABILITIES:GEOMETRY:END -->"
BEGIN_PROFILE = "<!-- CAPABILITIES:PROFILE:BEGIN -->"
END_PROFILE = "<!-- CAPABILITIES:PROFILE:END -->"
BEGIN_COUNT = "<!-- CAPABILITIES:SCAFFOLDCOUNT:BEGIN -->"
END_COUNT = "<!-- CAPABILITIES:SCAFFOLDCOUNT:END -->"
BEGIN_CENSUS = "<!-- CAPABILITIES:CENSUS:BEGIN -->"
END_CENSUS = "<!-- CAPABILITIES:CENSUS:END -->"

# A source file of this many lines or fewer is a doc-comment placeholder that
# reserves a name. The threshold is published on the page it generates.
STUB_MAX_LINES = 12

# Status is a human judgement, so it is declared rather than inferred from line
# counts. A crate absent from the published table lands here: `ifc-author` was
# documented as implemented in the capability list while missing from the
# census entirely, which the generated table surfaced.
PUBLISHED_STATUS = {
    # Both are documented as implemented in the capability list above the
    # census, with tests, yet neither had a census row.
    "ifc-author": '<span class="status-implemented">Implemented</span>',
    "ifc-spatial": '<span class="status-implemented">Implemented</span>',
    # All six PLAN tasks implemented: systems, ports, connectivity, flow,
    # zones and directed queries.
    "ifc-systems": '<span class="status-implemented">Implemented</span>',
    # Every PLAN task implemented, including PROP-EDIT on top of MODEL-MUT.
    "ifc-properties": '<span class="status-implemented">Implemented</span>',
    # VAL-STRUCT/TYPE/WHERE/REPORT implemented. Rules needing an EXPRESS
    # expression evaluator are registered as unsupported and reported, not
    # skipped, so a clean report never implies complete coverage.
    "ifc-validate": '<span class="status-implemented">Implemented</span>',
    # COST-RATE/REL/UNIT/CORPUS implemented over a validated fixture. COST-MUT
    # is deliberately open: authoring goes through ifc-model transactions, and
    # whether cost needs its own staging helpers is not yet decided.
    "ifc-cost": '<span class="status-partial">Partial</span>',
    # All six PLAN tasks implemented: plans, tasks, sequences, calendars,
    # events and timeline queries.
    "ifc-schedule": '<span class="status-implemented">Implemented</span>',
}


def schema_casing() -> dict[str, str]:
    """Map UPPERCASE entity names to their schema spelling."""
    text = CASING.read_text(encoding="utf-8")
    names = re.findall(r'"(Ifc\w+)"', text)
    return {n.upper(): n for n in names}


def const_list(source: str, name: str) -> list[str]:
    """Entity names inside a `pub const NAME: ... = &[ ... ];` table."""
    start = source.find(f"pub const {name}:")
    if start < 0:
        raise SystemExit(f"{name} not found")
    body = source[start:]
    end = None
    offset = 0
    for line in body.splitlines(keepends=True):
        stripped = line.strip()
        if stripped in ("];", ")];"):
            end = offset + len(line)
            break
        offset += len(line)
    if end is None:
        raise SystemExit(f"{name} table is not closed")
    return re.findall(r'"(IFC[A-Z0-9]+)"', body[:end])


def planned_details(source: str, name: str) -> dict[str, str]:
    """Entity -> reason, for a table of `(entity, reason)` tuples."""
    start = source.find(f"pub const {name}:")
    body = source[start:]
    end = None
    offset = 0
    for line in body.splitlines(keepends=True):
        stripped = line.strip()
        if stripped in ("];", ")];"):
            end = offset + len(line)
            break
        offset += len(line)
    table = body[:end]
    # Reasons may be split across lines with a trailing backslash.
    table = re.sub(r"\\\n\s*", "", table)
    pairs = re.findall(r'"(IFC[A-Z0-9]+)",\s*"((?:[^"\\]|\\.)*)"', table)
    return {e: r.replace('\\"', '"') for e, r in pairs}


def dispatched_profiles(source: str) -> tuple[list[str], dict[str, str]]:
    """Split profile match arms into ones that build and ones that refuse.

    An arm's presence is not a capability claim: `IfcArbitraryOpenProfileDef`
    has an arm whose whole body is a typed refusal. Reporting it as
    implemented because the name appears next to `=>` is precisely the
    substring reasoning that let the census overstate coverage.
    """
    lines = source.splitlines()
    built: list[str] = []
    refused: dict[str, str] = {}
    for index, raw in enumerate(lines):
        line = raw.strip()
        if not line.startswith('"'):
            continue
        name, _, rest = line[1:].partition('"')
        if not rest.lstrip().startswith("=>") or not name.startswith("IFC"):
            continue
        window = "\n".join(lines[index : index + 8])
        match = re.search(r'detail:\s*"((?:[^"\\]|\\.)*)"', window)
        if "GeometryError::Unsupported" in window and match:
            refused[name] = match.group(1)
        else:
            built.append(name)
    return built, refused


def geometry_table() -> str:
    source = DISPATCH.read_text(encoding="utf-8")
    casing = schema_casing()
    implemented = const_list(source, "IMPLEMENTED")
    planned = planned_details(source, "PLANNED")

    rows = ["| Family | Status |", "| --- | --- |"]
    for entity in implemented:
        rows.append(f"| `{casing.get(entity, entity)}` | {IMPLEMENTED} |")
    for entity, reason in planned.items():
        rows.append(f"| `{casing.get(entity, entity)}` | {PLANNED} \u2014 {reason} |")
    return "\n".join(rows)


def profile_table() -> str:
    source = PROFILE.read_text(encoding="utf-8")
    casing = schema_casing()
    dispatched, refused = dispatched_profiles(source)
    unlowered = planned_details(source, "UNLOWERED")
    unlowered.update(refused)

    rows = ["| Profile family | Status |", "| --- | --- |"]
    for entity in dispatched:
        if entity in unlowered:
            # The hardened coverage gate rejects this, but a generator that
            # silently picked one side would hide it from anyone reading docs.
            raise SystemExit(
                f"{entity} is both dispatched and declared unlowered"
            )
        rows.append(f"| `{casing.get(entity, entity)}` | {IMPLEMENTED} |")
    for entity, reason in unlowered.items():
        rows.append(f"| `{casing.get(entity, entity)}` | {PLANNED} \u2014 {reason} |")
    return "\n".join(rows)


def census_table(current: str) -> str:
    """Measure every crate in the workspace from its own sources.

    Status is not derived: it is a judgement, not a line count. `PUBLISHED_STATUS`
    is the source of truth and the page is the fallback, so promoting a crate
    means editing that map -- not editing generated output that will be
    overwritten on the next run.
    """
    previous = {
        name: status
        for name, status in re.findall(
            r"^\| `([a-z0-9-]+)` \|(?:[^|]*\|){4} (.+?) \|$", current, re.M
        )
    }
    if not previous:
        # First run after the sentinels replaced the literal table: recover the
        # published statuses from the last committed version of this page.
        import subprocess

        committed = subprocess.run(
            ["git", "show", f"HEAD:{TARGET.relative_to(ROOT)}"],
            cwd=ROOT,
            capture_output=True,
            text=True,
            check=False,
        ).stdout
        previous = {
            name: status
            for name, status in re.findall(
                r"^\| `([a-z0-9-]+)` \|(?:[^|]*\|){4} (.+?) \|$", committed, re.M
            )
        }
    rows = []
    for crate in sorted(ROOT.iterdir()):
        src = crate / "src"
        if not (crate / "Cargo.toml").exists() or not src.is_dir():
            continue
        files = sorted(src.rglob("*.rs"))
        loc = 0
        stubs = 0
        for path in files:
            lines = path.read_text(encoding="utf-8", errors="replace").splitlines()
            loc += len(lines)
            if len(lines) <= STUB_MAX_LINES:
                stubs += 1
        tests = len(list((crate / "tests").glob("*.rs"))) if (crate / "tests").is_dir() else 0
        # PUBLISHED_STATUS wins over the page: it is how a deliberate status
        # change is made. Reading the page first would make a crate's status
        # unchangeable, because the generator would keep restoring the old
        # value it just read back.
        status = PUBLISHED_STATUS.get(crate.name) or previous.get(crate.name)
        if status is None:
            raise SystemExit(
                f"{crate.name} has no published status; add it to PUBLISHED_STATUS "
                "before generating"
            )
        rows.append((crate.name, loc, len(files), stubs, tests, status))

    rows.sort(key=lambda r: r[1], reverse=True)
    out = [
        "| Crate | Source LOC | Files | Stub files | Test files | Status |",
        "| --- | ---: | ---: | ---: | ---: | --- |",
    ]
    for name, loc, files, stubs, tests, status in rows:
        out.append(f"| `{name}` | {loc:,} | {files} | {stubs} | {tests} | {status} |")
    return "\n".join(out)


def scaffold_count(census: str) -> str:
    """Sentence stating how many crates are scaffolds, counted from the table.

    Prose next to a generated table is exactly where drift reappears: the
    census was regenerated for months while the sentence beside it kept
    claiming a stale number.
    """
    scaffolds = census.count('status-scaffold')
    total = len(re.findall(r"^\| `[a-z0-9-]+` \|", census, re.M))
    return f"{scaffolds} of {total} crates are scaffolds."


def splice(text: str, begin: str, end: str, body: str) -> str:
    before, marker, rest = text.partition(begin)
    if not marker:
        raise SystemExit(f"missing {begin} sentinel in {TARGET}")
    _, marker, after = rest.partition(end)
    if not marker:
        raise SystemExit(f"missing {end} sentinel in {TARGET}")
    return f"{before}{begin}\n\n{body}\n\n{end}{after}"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--check",
        action="store_true",
        help="exit non-zero if the page is out of date instead of writing it",
    )
    args = parser.parse_args()

    current = TARGET.read_text(encoding="utf-8")
    updated = splice(current, BEGIN_CENSUS, END_CENSUS, census_table(current))
    updated = splice(updated, BEGIN_GEOMETRY, END_GEOMETRY, geometry_table())
    updated = splice(updated, BEGIN_PROFILE, END_PROFILE, profile_table())
    census = updated.split(BEGIN_CENSUS)[1].split(END_CENSUS)[0]
    updated = splice(updated, BEGIN_COUNT, END_COUNT, scaffold_count(census))

    if args.check:
        if current != updated:
            print(
                "docs/capabilities.md disagrees with the lowering source; "
                "run scripts/sync-capabilities.py",
                file=sys.stderr,
            )
            return 1
        print("capabilities in sync")
        return 0

    if current != updated:
        TARGET.write_text(updated, encoding="utf-8")
        print(f"updated {TARGET.relative_to(ROOT)}")
    else:
        print("capabilities already in sync")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
