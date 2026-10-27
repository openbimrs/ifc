#!/usr/bin/env bash
# Check the facade's `fresh_seed` under Node (#573).
#
#   crates/openbim-ifc/scripts/check-wasm-seed.sh
#
# Builds the `wasm_fresh_seed` example for wasm32-unknown-unknown twice, with
# and without `js-random`, binds both with wasm-bindgen for Node and runs
# check-wasm-seed.mjs: two processes draw different seeds with `js-random`,
# and the build without it refuses.
#
# The wasm-bindgen CLI must match the version pinned in this crate's
# Cargo.toml exactly, as for the npm package (see
# crates/openbim-ifc-wasm/scripts/build-npm-pkg.sh).
set -euo pipefail

crate_dir="$(cd "$(dirname "$0")/.." && pwd)"
root="$(cd "$crate_dir/../.." && pwd)"
cd "$root"

pinned="$(sed -n 's/^wasm-bindgen = "=\(.*\)"$/\1/p' "$crate_dir/Cargo.toml")"
if [[ -z "$pinned" ]]; then
    echo "error: wasm-bindgen is not pinned with = in $crate_dir/Cargo.toml" >&2
    exit 1
fi
if ! command -v wasm-bindgen >/dev/null; then
    echo "error: wasm-bindgen CLI missing; run: cargo install wasm-bindgen-cli --version $pinned --locked" >&2
    exit 1
fi
installed="$(wasm-bindgen --version | awk '{print $2}')"
if [[ "$installed" != "$pinned" ]]; then
    echo "error: wasm-bindgen CLI $installed != crate $pinned; run: cargo install wasm-bindgen-cli --version $pinned --locked --force" >&2
    exit 1
fi

module="${CARGO_TARGET_DIR:-$root/target}/wasm32-unknown-unknown/debug/examples/wasm_fresh_seed.wasm"
out="$(mktemp -d)"
trap 'rm -rf "$out"' EXIT

cargo build -p openbim-ifc --example wasm_fresh_seed --target wasm32-unknown-unknown --features js-random
wasm-bindgen --target nodejs --out-dir "$out/js-random" "$module"
cargo build -p openbim-ifc --example wasm_fresh_seed --target wasm32-unknown-unknown
wasm-bindgen --target nodejs --out-dir "$out/no-source" "$module"

node "$crate_dir/scripts/check-wasm-seed.mjs" "$out/js-random" "$out/no-source"
