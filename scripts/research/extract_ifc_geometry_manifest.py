#!/usr/bin/env python3
"""Extract the IFC geometry-resource declaration inventory from normative HTML.

The reference tree is intentionally not a build dependency. Run this script
manually when the pinned IFC schema baseline changes, then review and commit the
resulting TSV used by the Rust coverage test.
"""

from __future__ import annotations

import argparse
import html
import re
from pathlib import Path

RESOURCE_DIRS = (
    "ifcgeometryresource",
    "ifcgeometricmodelresource",
    "ifcgeometricconstraintresource",
)
KIND = re.compile(r'<span class="keyword">(ENTITY|TYPE|FUNCTION)</span>')
EXPRESS = re.compile(
    r'<div class="express"><code(?: class="express")?>(.*?)</code>', re.DOTALL
)
TAG = re.compile(r"<[^>]+>")


def declarations(schema_root: Path):
    for resource in RESOURCE_DIRS:
        lexical = schema_root / resource / "lexical"
        if not lexical.is_dir():
            raise SystemExit(f"missing normative lexical directory: {lexical}")
        for page in sorted(lexical.glob("*.htm")):
            source = page.read_text(encoding="utf-8", errors="strict")
            match = KIND.search(source)
            if not match:
                raise SystemExit(f"cannot identify declaration kind: {page}")
            express = EXPRESS.search(source)
            code = html.unescape(TAG.sub("", express.group(1) if express else ""))
            kind = match.group(1).lower()
            upper = code.upper()
            abstract = kind == "entity" and "ABSTRACT" in upper
            if kind == "type":
                if "SELECT" in upper:
                    subkind = "select"
                elif "ENUMERATION OF" in upper:
                    subkind = "enum"
                else:
                    subkind = "defined"
            else:
                subkind = "-"
            yield resource, kind, page.stem, abstract, subkind


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("schema_root", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    rows = list(declarations(args.schema_root))
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with args.output.open("w", encoding="utf-8", newline="\n") as stream:
        stream.write("resource\tkind\tname\tabstract\ttype_kind\n")
        for resource, kind, name, abstract, subkind in rows:
            stream.write(
                f"{resource}\t{kind}\t{name}\t{str(abstract).lower()}\t{subkind}\n"
            )
    print(f"wrote {len(rows)} declarations to {args.output}")


if __name__ == "__main__":
    main()
