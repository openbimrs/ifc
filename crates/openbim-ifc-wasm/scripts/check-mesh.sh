#!/usr/bin/env bash
# Build the opt-in mesh variant of the WebAssembly module (#328) and test it.
#
#   crates/openbim-ifc-wasm/scripts/check-mesh.sh
#
# The npm package leaves `mesh` out, so scripts/build-npm-pkg.sh proves only
# that `productMeshes` refuses there. This builds the module with
# `--features mesh` through the browser example's build.sh (which also
# binds it for the page, --target web), binds the same module for Node, and
# runs tests/js/geometry.mjs with IFC_WASM_MESH=1: the meshes, their typed
# arrays and refusals, and the example's scene code over them.
set -euo pipefail

crate_dir="$(cd "$(dirname "$0")/.." && pwd)"
root="$(cd "$crate_dir/../.." && pwd)"
target_dir="${CARGO_TARGET_DIR:-$root/target}"

"$crate_dir/examples/viewer/build.sh"

out="$(mktemp -d)"
trap 'rm -rf "$out"' EXIT
wasm-bindgen --target nodejs --out-dir "$out" \
    "$target_dir/wasm32-unknown-unknown/release/openbim_ifc_wasm.wasm"
# CommonJS glue; without its own package.json Node would take the docs
# site's "type": "module" (see build-npm-pkg.sh).
printf '{\n  "type": "commonjs"\n}\n' >"$out/package.json"

IFC_WASM_PKG="$out" IFC_WASM_MESH=1 node --test "$crate_dir/tests/js/geometry.mjs"
