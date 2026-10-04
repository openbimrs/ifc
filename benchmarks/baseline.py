#!/usr/bin/env python3
"""Pool and compare runs of the `baseline` bench (#14, #119).

    benchmarks/baseline.py summarize RUN.json [RUN.json ...]
    benchmarks/baseline.py compare --base RUN.json [...] --new RUN.json [...]

`summarize` pools several runs (processes) of the same build: for each
workload and benchmark it reports the median of all samples, their
interquartile range and median absolute deviation, the spread of the
per-run medians (run-to-run noise, which a single process cannot show),
and the heap figures. Runs whose checksums disagree are refused: they did
different work.

`compare` pools each side the same way and prints new/base for the
median. A change is called only when the two interquartile ranges do not
overlap; otherwise the row reads "within noise". It never judges a
threshold: it is for a person reading a change, not for CI.

Standard library only.
"""

import argparse
import json
import os
import statistics
import sys


def quantile(sorted_xs, q):
    pos = (len(sorted_xs) - 1) * q
    lo, hi = int(pos // 1), -int(-pos // 1)
    return sorted_xs[lo] + (sorted_xs[hi] - sorted_xs[lo]) * (pos - lo)


def summary(xs):
    xs = sorted(xs)
    median = quantile(xs, 0.5)
    mad = quantile(sorted(abs(x - median) for x in xs), 0.5)
    return {
        "median": median,
        "p25": quantile(xs, 0.25),
        "p75": quantile(xs, 0.75),
        "mad": mad,
        "min": xs[0],
        "max": xs[-1],
    }


def load_runs(paths):
    runs = []
    for path in paths:
        with open(path, encoding="utf-8") as handle:
            runs.append(json.load(handle))
    return runs


def pool(runs):
    """(workload, bench) -> pooled record, in first-seen order."""
    pooled = {}
    for run in runs:
        for r in run["results"]:
            key = (r["workload"], r["bench"])
            entry = pooled.setdefault(
                key,
                {
                    "entities": r["entities"],
                    "bytes": r["bytes"],
                    "checksum": r["checksum"],
                    "ms": [],
                    "run_medians": [],
                    "heap": [],
                },
            )
            if entry["checksum"] != r["checksum"]:
                sys.exit(f"error: {key} checksum differs between runs: different work")
            entry["ms"].extend(r["ms"])
            entry["run_medians"].append(statistics.median(r["ms"]))
            if r["heap_bytes"] is not None:
                entry["heap"].append(r["heap_bytes"])
    return pooled


def fmt(x):
    return f"{x:.4f}" if x < 1 else f"{x:.2f}" if x < 100 else f"{x:.0f}"


def mb(x):
    return f"{x / 1048576:.2f}"


def summarize(args):
    runs = load_runs(args.runs)
    env = runs[0]["environment"]
    print(f"Runs: {len(runs)} processes.")
    for key in (
        "cpu",
        "threads available",
        "cpus allowed",
        "memory",
        "os",
        "kernel",
        "profile",
        "warm-up",
        "samples",
        "probe",
    ):
        print(f"- {key}: {env.get(key, 'unknown')}")
    for path, run in zip(args.runs, runs):
        e = run["environment"]
        name = os.path.basename(path)
        print(f"- {name}: load before {e['load before']}, after {e['load after']}")
    print()
    print(
        "| workload | entities | bench | median ms | IQR ms | MAD ms | "
        "run medians ms | heap retained MB | heap peak MB |"
    )
    print("| --- | ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: |")
    for (workload, bench), e in pool(runs).items():
        s = summary(e["ms"])
        runs_lo, runs_hi = min(e["run_medians"]), max(e["run_medians"])
        retained = peak = ""
        if e["heap"]:
            retained = mb(statistics.median(h["retained"] for h in e["heap"]))
            peak = mb(statistics.median(h["peak"] for h in e["heap"]))
        print(
            f"| {workload} | {e['entities']:,} | {bench} | {fmt(s['median'])} | "
            f"{fmt(s['p25'])}..{fmt(s['p75'])} | {fmt(s['mad'])} | "
            f"{fmt(runs_lo)}..{fmt(runs_hi)} | {retained} | {peak} |"
        )


def compare(args):
    base, new = pool(load_runs(args.base)), pool(load_runs(args.new))
    print("| workload | bench | base median ms | new median ms | new/base | verdict |")
    print("| --- | --- | ---: | ---: | ---: | --- |")
    for key, b in base.items():
        n = new.get(key)
        if n is None:
            continue
        workload, bench = key
        if n["checksum"] != b["checksum"]:
            print(f"| {workload} | {bench} | | | | different output: not comparable |")
            continue
        sb, sn = summary(b["ms"]), summary(n["ms"])
        ratio = sn["median"] / sb["median"]
        if sn["p75"] < sb["p25"]:
            verdict = "faster"
        elif sn["p25"] > sb["p75"]:
            verdict = "slower"
        else:
            verdict = "within noise"
        print(
            f"| {workload} | {bench} | {fmt(sb['median'])} | {fmt(sn['median'])} | "
            f"{ratio:.3f} | {verdict} |"
        )


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    sub = parser.add_subparsers(dest="command", required=True)
    s = sub.add_parser("summarize", help="pool runs into one table")
    s.add_argument("runs", nargs="+")
    c = sub.add_parser("compare", help="compare pooled runs against a baseline")
    c.add_argument("--base", nargs="+", required=True)
    c.add_argument("--new", nargs="+", required=True)
    args = parser.parse_args()
    {"summarize": summarize, "compare": compare}[args.command](args)


if __name__ == "__main__":
    main()
