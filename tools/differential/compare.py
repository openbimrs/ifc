#!/usr/bin/env python3
"""Join our differential output against the IfcOpenShell reference."""
import json, sys, collections

REL_TOL = 1e-6


def load(path):
    out = {}
    for line in open(path):
        line = line.strip()
        if not line:
            continue
        r = json.loads(line)
        if "id" in r:
            out[(r["file"], r["id"])] = r
    return out


def agree(a, b):
    """Relative volume agreement; ordering differences make bitwise equality wrong."""
    scale = max(abs(a), abs(b), 1e-12)
    return abs(a - b) / scale <= REL_TOL


def join(ours_path, ref_path):
    ours, ref = load(ours_path), load(ref_path)
    ours_keys = set(ours)
    ref_keys = set(ref)
    both = sorted(ours_keys.intersection(ref_keys))
    only_ref = sorted(ref_keys.difference(ours_keys))
    only_ours = sorted(ours_keys.difference(ref_keys))
    rows, mismatches = [], []
    for key in both:
        o, r = ours[key], ref[key]
        if "error" in r or "volume" not in r:
            continue
        ok = agree(o["volume"], r["volume"])
        rows.append((key, o, r, ok))
        if not ok:
            mismatches.append((key, o, r))
    return rows, mismatches, only_ref, only_ours


def main():
    if len(sys.argv) != 3:
        print("usage: compare.py <ours.jsonl> <reference.jsonl>", file=sys.stderr)
        return 2
    rows, mismatches, only_ref, only_ours = join(sys.argv[1], sys.argv[2])
    print("# Differential report: nehirde vs IfcOpenShell")
    print()
    print(f"Products compared: {len(rows)}")
    print(f"Volume agreement:  {len(rows) - len(mismatches)}/{len(rows)}")
    print(f"Only in reference: {len(only_ref)}")
    print(f"Only in ours:      {len(only_ours)}")
    print()
    print("| file | id | type | ours | reference | rel.diff | manifold | ours ms | ref ms |")
    print("|---|---|---|---|---|---|---|---|---|")
    for (f, i), o, r, ok in rows:
        scale = max(abs(o["volume"]), abs(r["volume"]), 1e-12)
        rel = abs(o["volume"] - r["volume"]) / scale
        mark = "ok" if ok else "MISMATCH"
        man = "both" if o["manifold"] and r["manifold"] else "DIFFER"
        cells = [f, str(i), o['type'], f"{o['volume']:.6f}", f"{r['volume']:.6f}", f"{rel:.2e} {mark}", man, f"{o['ms']:.2f}", f"{r['ms']:.2f}"]
        print('| ' + ' | '.join(cells) + ' |')
    return 0


if __name__ == "__main__":
    sys.exit(main())
