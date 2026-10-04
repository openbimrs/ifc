#!/usr/bin/env bash
# Measure the codec and entity-graph baselines (#14, #119) on this machine.
#
#   benchmarks/run-baseline.sh [out-dir]
#
# Builds the `baseline` bench once (release settings), then runs it RUNS
# times per scale, each run its own process pinned with taskset, rounds
# interleaved so that slow drift spreads over every scale. A run starts only
# when the 1-minute load is below MAX_LOAD, and is discarded and repeated
# when the load at its end exceeds END_MAX_LOAD (the bench itself adds about
# one). Nothing may build while it measures; this script builds first.
#
# Environment (defaults):
#   RUNS=3  SCALES="fixtures small crossover large"  CPUS=12-19
#   MAX_LOAD=3  END_MAX_LOAD=4  WAIT=60 (s between load checks)
#   MAX_WAITS=120 (checks before giving up)  ATTEMPTS=3 (per run)
#   SAMPLES, WARMUP: passed to the bench when set (the bench's own
#   defaults are 20 samples after 3 warm-up)
#   LARGE_SAMPLES=10, LARGE_WARMUP=2: the large scale's plan; one of its
#   samples takes up to two seconds plus setup, so 20 would take ~10 min
#   IFC_BENCH_DATA: where generated synthetic files go (default out-dir/data)
#   PROVISIONAL=1: skip the load gate and label the result provisional
#
# Output: out-dir/run<R>-<scale>.{json,md}, out-dir/environment.txt and
# out-dir/summary.md (benchmarks/baseline.py summarize over every run).
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
out="${1:-${CARGO_TARGET_DIR:-$root/target}/bench-baseline}"
data="${IFC_BENCH_DATA:-$out/data}"
runs="${RUNS:-3}"
scales="${SCALES:-fixtures small crossover large}"
cpus="${CPUS:-12-19}"
max_load="${MAX_LOAD:-3}"
end_max_load="${END_MAX_LOAD:-4}"
wait_s="${WAIT:-60}"
max_waits="${MAX_WAITS:-120}"
attempts="${ATTEMPTS:-3}"
mkdir -p "$out" "$out/discarded" "$data"

bin="$(cd "$root" && cargo bench -p ifc-step --bench baseline --no-run --message-format=json |
    python3 -c '
import json, sys
for line in sys.stdin:
    m = json.loads(line)
    if m.get("reason") == "compiler-artifact" and m["target"]["name"] == "baseline" and m.get("executable"):
        print(m["executable"])
')"
[[ -x "$bin" ]] || { echo "error: bench binary not found" >&2; exit 1; }

{
    echo "date: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
    echo "commit: $(git -C "$root" rev-parse HEAD)$(git -C "$root" diff --quiet || echo ' (dirty)')"
    echo "cpu: $(grep -m1 'model name' /proc/cpuinfo | cut -d: -f2- | sed 's/^ //')"
    echo "logical cpus: $(nproc --all), pinned to: $cpus"
    echo "memory: $(grep MemTotal /proc/meminfo | awk '{printf "%.0f GiB", $2/1048576}')"
    echo "os: $(. /etc/os-release && echo "$PRETTY_NAME"), kernel $(uname -r)"
    echo "rustc: $(cd "$root" && rustc -V)"
    echo "profile: bench (release: opt-level 3, thin LTO, 1 codegen unit)"
    echo "load gate: start below $max_load, discard above $end_max_load at end"
    [[ -n "${PROVISIONAL:-}" ]] && echo "PROVISIONAL: load gate off"
} >"$out/environment.txt"
cat "$out/environment.txt"

load1() { cut -d' ' -f1 /proc/loadavg; }
below() { awk -v l="$1" -v m="$2" 'BEGIN { exit !(l < m) }'; }

wait_quiet() {
    [[ -n "${PROVISIONAL:-}" ]] && return 0
    for ((i = 0; i < max_waits; i++)); do
        below "$(load1)" "$max_load" && return 0
        echo "  load $(load1) >= $max_load; waiting ${wait_s}s ($((i + 1))/$max_waits)" >&2
        sleep "$wait_s"
    done
    echo "error: the machine never got quiet; rerun later or set PROVISIONAL=1" >&2
    exit 3
}

plan() {
    if [[ "$1" == large ]]; then
        echo --samples "${LARGE_SAMPLES:-10}" --warmup "${LARGE_WARMUP:-2}"
    else
        [[ -n "${SAMPLES:-}" ]] && echo -n "--samples $SAMPLES "
        [[ -n "${WARMUP:-}" ]] && echo -n "--warmup $WARMUP"
        echo
    fi
}

for ((run = 1; run <= runs; run++)); do
    for scale in $scales; do
        name="run$run-$scale"
        for ((attempt = 1; ; attempt++)); do
            wait_quiet
            echo "== $name (attempt $attempt, load $(load1))" >&2
            # shellcheck disable=SC2046 # the plan is a word list
            taskset -c "$cpus" "$bin" --bench --scale "$scale" --data-dir "$data" \
                --json "$out/$name.json" $(plan "$scale") >"$out/$name.md"
            end="$(load1)"
            if [[ -n "${PROVISIONAL:-}" ]] || below "$end" "$end_max_load"; then
                break
            fi
            echo "  load $end at end >= $end_max_load: discarded" >&2
            mv "$out/$name.json" "$out/discarded/$name-$attempt.json"
            mv "$out/$name.md" "$out/discarded/$name-$attempt.md"
            if ((attempt >= attempts)); then
                echo "error: $name stayed noisy after $attempts attempts" >&2
                exit 3
            fi
        done
    done
done

python3 "$root/benchmarks/baseline.py" summarize "$out"/run*-*.json >"$out/summary.md"
echo "summary: $out/summary.md"
