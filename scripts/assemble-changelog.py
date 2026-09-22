#!/usr/bin/env python3
"""Assemble the documentation changelog from per-crate changelogs.

Each crate owns its CHANGELOG.md and versions independently, so there is no
single file to mirror. This script reads every per-crate changelog, groups
their entries by release version, and writes one aggregated page for the
docs site between sentinel comments.

Ordering is by semantic version descending, so the newest release across the
whole family leads the page regardless of which crate produced it.

Run with ``--check`` in CI to fail when the page has drifted.
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
TARGET = ROOT / "docs" / "project" / "changelog.md"
ARCHIVE = ROOT / "CHANGELOG.md"
BLOB = "https://github.com/openbimrs/ifc/blob/main"
BEGIN = "<!-- CHANGELOG:BEGIN -->"
END = "<!-- CHANGELOG:END -->"

# A release heading: "## [0.2.1] - 2026-09-23", optionally "[Unreleased]".
HEADING = re.compile(r"^## \[([^\]]+)\](?:\s*-\s*(\S+))?\s*$")


def members() -> list[str]:
    """Every publishable workspace member, from cargo itself.

    Deriving this from metadata rather than a hand-kept list means a new
    crate is picked up the moment it joins the workspace.
    """
    out = subprocess.run(
        ["cargo", "metadata", "--no-deps", "--format-version", "1"],
        cwd=ROOT, capture_output=True, text=True, check=True,
    ).stdout
    data = json.loads(out)
    names = [
        p["name"]
        for p in data["packages"]
        if p.get("publish") != []
    ]
    return sorted(names)


def parse(text: str) -> list[tuple[str, str, str]]:
    """Split a changelog into (version, date, body) triples.

    Link-reference lines at the foot ("[0.2.0]: https://...") belong to the
    source file, not to any release body, so they are dropped: the assembled
    page defines its own anchors.
    """
    sections: list[tuple[str, str, str]] = []
    version = date = None
    body: list[str] = []
    for line in text.splitlines():
        match = HEADING.match(line)
        if match:
            if version is not None:
                sections.append((version, date or "", "\n".join(body).strip()))
            version, date = match.group(1), match.group(2)
            body = []
            continue
        if version is not None and not re.match(r"^\[[^\]]+\]:\s+\S+", line):
            body.append(line)
    if version is not None:
        sections.append((version, date or "", "\n".join(body).strip()))
    return sections


def absolutise(body: str) -> str:
    """Rewrite repo-relative links to absolute GitHub URLs.

    A per-crate changelog is read in two places: in the repository,
    where `../CHANGELOG.md` resolves, and on the docs site, where it
    does not. VitePress treats the latter as a dead link and fails the
    build, so relative targets are absolutised on the way in.
    """
    def repl(match: re.Match) -> str:
        text, target = match.group(1), match.group(2)
        if "://" in target or target.startswith("#"):
            return match.group(0)
        clean = target.lstrip("./")
        return f"[{text}]({BLOB}/{clean})"
    return re.sub(r"\[([^\]]+)\]\(([^)]+)\)", repl, body)

def version_key(version: str) -> tuple:
    """Sort key placing Unreleased first, then versions newest-first."""
    if version.lower() == "unreleased":
        return (1, ())
    parts = []
    for piece in re.split(r"[.\-+]", version):
        parts.append(int(piece) if piece.isdigit() else 0)
    return (0, tuple(parts))


def assemble() -> str:
    """Group every crate section by version, newest release first."""
    by_version: dict[str, list[tuple[str, str, str]]] = {}
    for name in members():
        path = ROOT / name / "CHANGELOG.md"
        if not path.exists():
            continue
        for version, date, body in parse(path.read_text(encoding="utf-8")):
            if not body:
                # An empty Unreleased section is the normal resting state;
                # listing the crate with nothing under it is just noise.
                continue
            by_version.setdefault(version, []).append((name, date, absolutise(body)))

    blocks: list[str] = []
    for version in sorted(by_version, key=version_key, reverse=True):
        entries = sorted(by_version[version])
        dates = {d for _, d, _ in entries if d}
        date = f" - {sorted(dates)[-1]}" if dates else ""
        blocks.append(f"## [{version}]{date}")
        blocks.append("")
        for name, _, body in entries:
            blocks.append(f"### {name}")
            blocks.append("")
            blocks.append(body)
            blocks.append("")
    return "\n".join(blocks).strip()


def render(target_text: str, body: str) -> str:
    before, _, rest = target_text.partition(BEGIN)
    if not rest:
        raise SystemExit(f"missing {BEGIN} sentinel in {TARGET}")
    _, _, after = rest.partition(END)
    if not after and END not in rest:
        raise SystemExit(f"missing {END} sentinel in {TARGET}")
    return f"{before}{BEGIN}\n\n{body}\n\n{END}{after}"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--check",
        action="store_true",
        help="exit non-zero if the docs page is out of date instead of writing it",
    )
    args = parser.parse_args()

    missing = [n for n in members() if not (ROOT / n / "CHANGELOG.md").exists()]
    if missing:
        print(
            "crates without a CHANGELOG.md: " + ", ".join(missing),
            file=sys.stderr,
        )
        return 1

    body = assemble()
    current = TARGET.read_text(encoding="utf-8")
    updated = render(current, body)

    if args.check:
        if current != updated:
            print(
                "docs/project/changelog.md is out of date; "
                "run scripts/assemble-changelog.py",
                file=sys.stderr,
            )
            return 1
        print("changelog in sync")
        return 0

    if current != updated:
        TARGET.write_text(updated, encoding="utf-8")
        print(f"updated {TARGET.relative_to(ROOT)}")
    else:
        print("changelog already in sync")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
