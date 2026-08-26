#!/usr/bin/env python3
"""Synchronise the canonical root CHANGELOG.md into the documentation site.

A changelog that must be updated in two places drifts. The root file is
canonical because it is what appears on GitHub and in the published crate; this
script copies its body into the docs page between sentinel comments.

Run with ``--check`` in CI to fail when the two have diverged.
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "CHANGELOG.md"
TARGET = ROOT / "docs" / "project" / "changelog.md"
BEGIN = "<!-- CHANGELOG:BEGIN -->"
END = "<!-- CHANGELOG:END -->"

# The docs page supplies its own H1 and provenance note, so the source's
# leading title and Keep-a-Changelog preamble would be duplicated content.
SKIP_PREFIXES = ("# ",)


def changelog_body(text: str) -> str:
    """Return the changelog from its first release section onward."""
    lines = text.splitlines()
    for index, line in enumerate(lines):
        if line.startswith("## "):
            return "\n".join(lines[index:]).strip()
    # No release section yet: fall back to everything below the title.
    kept = [line for line in lines if not line.startswith(SKIP_PREFIXES)]
    return "\n".join(kept).strip()


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

    if not SOURCE.exists():
        raise SystemExit(f"canonical changelog not found: {SOURCE}")

    body = changelog_body(SOURCE.read_text(encoding="utf-8"))
    current = TARGET.read_text(encoding="utf-8")
    updated = render(current, body)

    if args.check:
        if current != updated:
            print(
                "docs/project/changelog.md is out of date with CHANGELOG.md; "
                "run scripts/sync-changelog.py",
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
