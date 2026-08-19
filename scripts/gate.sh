#!/usr/bin/env bash
# Full gate for nehirde. Trusts EXIT CODES, not parsed output.
#
# Why this exists: an earlier ad-hoc `cargo test ... | grep "test result" | awk`
# pipeline reported "0 failed" while a test was in fact failing. Two bugs:
#   1. piping cargo into grep discards cargo's exit status ($? is grep's);
#   2. `awk -F'[ ;]'` on "ok. 4 passed; 0 failed" puts the failed count in $7,
#      not $6 -- $6 is the empty field between ';' and ' '.
# Never parse counts to decide pass/fail. Check the exit code.
set -uo pipefail
export PATH="$HOME/.cargo/bin:$PATH"
cd "$(dirname "$0")/.."

gate_out="$(mktemp "${TMPDIR:-/tmp}/nehirde-gate.XXXXXX")" || exit 1
trap 'rm -f "$gate_out"' EXIT

fail=0
step() {
    local name="$1"; shift
    printf '%-46s' "$name"
    if "$@" >"$gate_out" 2>&1; then
        echo "ok"
    else
        echo "FAIL (exit $?)"
        tail -25 "$gate_out" | sed 's/^/    /'
        fail=1
    fi
}

echo "=== nehirde gate ==="
step "fmt --check"            cargo fmt --all -- --check
step "build --workspace"      cargo build --workspace
step "test --workspace"       cargo test --workspace
step "test --all-features"    cargo test --workspace --all-features
step "clippy"                 cargo clippy --workspace --all-targets -- -D warnings
step "clippy --all-features"  cargo clippy --workspace --all-targets --all-features -- -D warnings
step "doc"                    env RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps

# The ifc facade must build and lint under each feature combination.
for f in "--no-default-features" "--features step" "--features ifcxml" "--all-features"; do
    # shellcheck disable=SC2086
    step "ifc build $f"  cargo build -p ifc $f
    # shellcheck disable=SC2086
    step "ifc clippy $f" cargo clippy -p ifc $f --all-targets -- -D warnings
done

# --- Geometry capability and isolation matrices. ----------------------------
# Concrete backends are separate packages, so Cargo feature unification cannot
# make them visible through `geom-kernel`. Isolated builds still prove each
# package declares its own complete dependency set.
step "geometry feature matrix" scripts/geometry-feature-matrix.sh

geometry_crates="geom-core geom-mesh geom-profile geom-curve geom-surface \
geom-topology geom-model geom-primitive geom-sweep geom-tessellate geom-spatial \
geom-measure geom-heal geom-kernel geom-backend-cpu geom-backend-gpu geom"
for c in ifc-geometry ifc-alignment ifc-georef ifc-model $geometry_crates; do
    step "isolated build -p $c" cargo build -p "$c"
done

echo
if [ "$fail" -eq 0 ]; then
    echo "GATE PASSED"
else
    echo "GATE FAILED"
fi
exit "$fail"
