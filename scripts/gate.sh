#!/usr/bin/env bash
# Complete standalone verification gate for openbimrs/ifc.
set -euo pipefail

cd "$(dirname "$0")/.."

# Shared fleet workspaces reuse a global target directory. Isolate this
# standalone repository so compile-time manifest paths cannot leak in from the
# superproject build of the same package names.
if [[ -z "${CARGO_TARGET_DIR:-}" && -d /mnt/backup/build-cache ]]; then
    export CARGO_TARGET_DIR=/mnt/backup/build-cache/openbim-ifc-standalone
fi

cargo fmt --all -- --check
cargo build --workspace --all-targets
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps

cargo test -p ifc-model --test package_architecture
cargo test -p ifc-model --test progressive_context
cargo test -p ifc-model --test module_reachability
cargo test -p ifc-model --test no_monolithic_files
cargo test -p ifc-geometry --test declaration_manifest
cargo test -p ifc-geometry --test no_backend_dependency

for features in "--no-default-features" "--features step" "--features ifcxml" "--all-features"; do
    # shellcheck disable=SC2086
    cargo build -p openbim-ifc $features
    # shellcheck disable=SC2086
    cargo clippy -p openbim-ifc $features --all-targets -- -D warnings
done
