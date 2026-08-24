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

# The openbim-ifc facade must build and lint under each feature combination.
for f in "--no-default-features" "--features step" "--features ifcxml" "--all-features"; do
    # shellcheck disable=SC2086
    step "openbim-ifc build $f"  cargo build -p openbim-ifc $f
    # shellcheck disable=SC2086
    step "openbim-ifc clippy $f" cargo clippy -p openbim-ifc $f --all-targets -- -D warnings
done

# The openbim facade must build and lint under each standard in isolation.
# This is the executable form of ADR 0015's central claim: enabling one
# standard must not drag in another's dependencies.
for f in "--no-default-features" \
         "--no-default-features --features dt" \
         "--no-default-features --features ids" \
         "--no-default-features --features bcf" \
         "--no-default-features --features icdd" \
         "--no-default-features --features idm" \
         "--no-default-features --features loin" \
         "--no-default-features --features full"; do
    # shellcheck disable=SC2086
    step "openbim build $f"  cargo build -p openbim $f
    # shellcheck disable=SC2086
    step "openbim clippy $f" cargo clippy -p openbim $f --all-targets -- -D warnings
done

# Isolated builds prove each crate declares its own complete dependency set:
# feature unification inside a workspace build can otherwise hide a missing
# dependency that only shows up for an external consumer.
for c in openbim-codec-xml openbim-codec-zip openbim-ifc openbim-core openbim-dt openbim-ids openbim-bcf \
         openbim-icdd openbim-idm openbim-loin openbim clash diff \
         icdd idmxml loin; do
    step "isolated build -p $c" cargo build -p "$c"
done

# Alias crates must stay pure re-exports. A type defined in an alias would be
# distinct from -- and non-unifiable with -- the canonical crate's type, so a
# graph holding both would not compile. Guard the invariant structurally.
step "alias crates define no types" scripts/check-alias-purity.sh

# Geometry-kernel feature and layering gates live in the separate Axiolid repository.

echo
if [ "$fail" -eq 0 ]; then
    echo "GATE PASSED"
else
    echo "GATE FAILED"
fi
exit "$fail"
