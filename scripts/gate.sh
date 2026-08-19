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

fail=0
step() {
    local name="$1"; shift
    printf '%-46s' "$name"
    if "$@" >/tmp/gate_out.txt 2>&1; then
        echo "ok"
    else
        echo "FAIL (exit $?)"
        tail -25 /tmp/gate_out.txt | sed 's/^/    /'
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

# --- Isolated per-crate builds. ---------------------------------------------
# `cargo build --workspace` UNIFIES features: apps/ifc-cli enables geom-kernel's
# `scalar`+`simd`, which switches those features on for every crate in the same
# resolve. A crate under packages/ifc/ can therefore CALL a backend impl
# (geom_kernel::backend::scalar::...) and the workspace build stays green, even
# though its own manifest says default-features = false.
#
# Verified, not theoretical: adding that exact line to ifc-geometry passed
# `build --workspace` while `build -p ifc-geometry` failed with E0433, and the
# manifest-reading test could not see it (it inspects Cargo.toml, not code).
#
# Building each crate ALONE resolves features for that crate only, so the leak
# becomes a compile error. This is the check that makes the kernel swap real.
for c in ifc-geometry ifc-alignment ifc-georef ifc-model geom-core geom-mesh; do
    step "isolated build -p $c" cargo build -p "$c"
done

# The contract must compile with no backend behind it at all.
step "geom-kernel contract only" cargo build -p geom-kernel --no-default-features

echo
if [ "$fail" -eq 0 ]; then
    echo "GATE PASSED"
else
    echo "GATE FAILED"
fi
exit "$fail"
