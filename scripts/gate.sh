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
cargo test -p ifc-geometry --test kernel_free_build

# The kernel-free column. `--all-features` above cannot see a boundary that
# only exists when a feature is OFF, so a 2D consumer's build is verified
# explicitly: it must compile, pass its tests, and link no geometry crate.
cargo build -p ifc-geometry --no-default-features
cargo test -p ifc-geometry --no-default-features
cargo clippy -p ifc-geometry --no-default-features --all-targets -- -D warnings
# Intra-doc links to feature-gated items resolve under `--all-features` and
# break here, so rustdoc gets its own kernel-free run.
RUSTDOCFLAGS="-D warnings" cargo doc -p ifc-geometry --no-default-features --no-deps

for features in "--no-default-features" "--features step" "--features ifcxml" "--features step,geometry-select" "--all-features"; do
    # shellcheck disable=SC2086
    cargo build -p openbim-ifc $features
    # shellcheck disable=SC2086
    cargo clippy -p openbim-ifc $features --all-targets -- -D warnings
done

# Documentation gates. The changelog page is generated from the canonical root
# CHANGELOG.md, so drift between them is a build failure rather than a silent
# inconsistency the reader has to notice.
python3 scripts/sync-changelog.py --check

# Licensing gate. The IFC schemas are CC BY-ND 4.0 and must never reach the
# published tree; this rejects XSD/PDF payloads and any `references/` or
# `schemas/` path. It ran only by hand until now, which is how a tracked
# `ifc-geometry/references/` survived undetected.
python3 scripts/check-leakage.py
