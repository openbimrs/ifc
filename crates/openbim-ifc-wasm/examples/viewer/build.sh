#!/usr/bin/env bash
# Build the mesh-enabled WebAssembly module the viewer imports (#328).
#
#   crates/openbim-ifc-wasm/examples/viewer/build.sh [--serve]
#
# The npm package leaves meshes out (`productMeshes` throws
# `feature-disabled`), so this builds the crate with `--features mesh` and
# binds it with `wasm-bindgen --target web` into ./pkg (gitignored). With
# --serve it then serves the repository root, so the page can fetch the
# default fixture from test/fixtures, and prints the page's URL.
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
root="$(cd "$here/../../../.." && pwd)"
target_dir="${CARGO_TARGET_DIR:-$root/target}"

(cd "$root" && cargo build -p openbim-ifc-wasm --target wasm32-unknown-unknown --release --features mesh)
rm -rf "$here/pkg"
wasm-bindgen --target web --out-dir "$here/pkg" \
    "$target_dir/wasm32-unknown-unknown/release/openbim_ifc_wasm.wasm"

if [[ "${1:-}" == "--serve" ]]; then
    echo "open http://localhost:8000/crates/openbim-ifc-wasm/examples/viewer/"
    exec python3 -m http.server --directory "$root" 8000
fi
