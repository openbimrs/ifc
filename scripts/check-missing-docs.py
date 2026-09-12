#!/usr/bin/env python3
"""Ratchet the documentation debt downward.

Twelve crates document every public item and enforce it permanently through
`[workspace.lints] missing_docs = "deny"`. The rest carry a measured debt and
`#![allow(missing_docs)]`; this script pins that debt so it can only shrink.

The lint is measured with `--force-warn`, which reports through the crate-level
`allow`. Without that the allow would hide the very thing being counted and the
budget would read zero for every crate.

Usage:
    check-missing-docs.py            # fail if any crate exceeds its budget
    check-missing-docs.py --update   # rewrite budgets to the measured counts
"""

from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

# Measured ceilings. A crate may never exceed its entry; lowering one is the
# point of the exercise. Reaching zero means the crate can move onto
# [workspace.lints] and leave this table for good.
BUDGET = {
}


def enforced_crates() -> list[str]:
    """Workspace crates that opt into [workspace.lints]."""
    found = []
    for manifest in sorted(ROOT.glob("*/Cargo.toml")):
        if "[lints]" in manifest.read_text():
            found.append(manifest.parent.name)
    return found


def measure(crate: str) -> int:
    """Count undocumented public items in `crate`.

    `--force-warn` overrides the crate's own `allow(missing_docs)`, and the
    source file is touched first because cargo will not re-lint a fresh cache.

    `--all-features` matters: a feature-gated module is invisible to a
    default-feature measurement, so its undocumented items would go uncounted
    and the crate could be promoted while still holding real debt.
    """
    lib = ROOT / crate / "src" / "lib.rs"
    if lib.exists():
        lib.touch()
    result = subprocess.run(
        [
            "cargo",
            "rustc",
            "-p",
            crate,
            "--lib",
            "--all-features",
            "--",
            "--force-warn",
            "missing_docs",
        ],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=False,
    )
    if result.returncode != 0 and "missing documentation" not in result.stderr:
        print(f"{crate}: build failed\n{result.stderr}", file=sys.stderr)
        sys.exit(2)
    return result.stderr.count("missing documentation")


def main() -> int:
    update = "--update" in sys.argv
    measured: dict[str, int] = {}
    failures: list[str] = []
    wins: list[str] = []

    for crate in enforced_crates():
        if measure(crate):
            failures.append(
                f"  {crate}: enforces missing_docs but reports undocumented "
                f"items; the lint and this check disagree"
            )

    for crate in sorted(BUDGET):
        count = measure(crate)
        measured[crate] = count
        allowed = BUDGET[crate]
        if count > allowed:
            failures.append(
                f"  {crate}: {count} undocumented items, budget is {allowed} "
                f"(+{count - allowed}); document the new items or raise nothing"
            )
        elif count < allowed:
            wins.append(f"  {crate}: {count} < {allowed}, lower the budget by {allowed - count}")

    if update:
        rewrite(measured)
        print("budgets updated")
        return 0

    for crate, count in sorted(measured.items()):
        print(f"  {crate:<24} {count:>4} / {BUDGET[crate]}")
    print(f"total: {sum(measured.values())} undocumented public items")
    print(f"enforced: {len(enforced_crates())} crates deny missing_docs")

    if failures:
        print("\ndocumentation debt grew:", file=sys.stderr)
        print("\n".join(failures), file=sys.stderr)
        return 1

    if wins:
        print("\ndebt shrank -- run scripts/check-missing-docs.py --update:")
        print("\n".join(wins))
        return 1

    return 0


def rewrite(measured: dict[str, int]) -> None:
    """Write measured counts back into this file's BUDGET table."""
    path = Path(__file__).resolve()
    text = path.read_text()
    body = "\n".join(f'    "{c}": {n},' for c, n in sorted(measured.items()) if n)
    updated = re.sub(
        r"BUDGET = \{\n.*?\n\}",
        f"BUDGET = {{\n{body}\n}}",
        text,
        flags=re.DOTALL,
    )
    path.write_text(updated)


if __name__ == "__main__":
    raise SystemExit(main())
