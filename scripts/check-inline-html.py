#!/usr/bin/env python3
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CONFIG = ROOT / "docs/.vitepress/config.ts"
CAPS = ROOT / "docs/capabilities.md"

def main() -> int:
    cfg = CONFIG.read_text(encoding="utf-8")
    # Ignore comment lines so prose explaining the setting cannot trip this.
    live = "\n".join(
        line for line in cfg.splitlines() if not line.lstrip().startswith("//")
    )
    if re.search(r"html:\s*false", live):
        print("docs/.vitepress/config.ts sets markdown.html=false;")
        print("generated status badges and CAPABILITIES markers would")
        print("render as escaped text. Set html: true.")
        return 1
    if "<span class=" not in CAPS.read_text(encoding="utf-8"):
        print("docs/capabilities.md has no status badges; regenerate it.")
        return 1
    print("inline html ok")
    return 0

if __name__ == "__main__":
    sys.exit(main())
