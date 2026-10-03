#!/usr/bin/env bash
# Build the @openbim/ifc npm package, then test every target it ships.
#
#   crates/openbim-ifc-wasm/scripts/build-npm-pkg.sh [out-dir]
#
# One release build of the wasm module, bound three times by wasm-bindgen
# and shrunk by `wasm-opt -Oz`:
#
#   <out>/            --target nodejs   CommonJS for Node (the `node` condition)
#   <out>/bundler/    --target bundler  ES module for webpack, Vite, Rollup
#   <out>/web/        --target web      ES module for a browser, no bundler
#
# Then the Node smoke and corpus suites run against <out>, and
# tools/check-package.mjs packs <out> as npm would publish it and checks each
# target from that tarball: Node `require` and `import`, a webpack bundle,
# and both browser builds in headless Chrome (#40).
#
# The wasm-bindgen CLI must match the `wasm-bindgen` crate version pinned in
# Cargo.toml exactly: a mismatch fails at bindgen time with a schema error,
# or worse, generates glue for a different ABI. This script refuses early and
# names the version to install. wasm-opt and webpack come pinned from
# tools/package-lock.json.
set -euo pipefail

crate_dir="$(cd "$(dirname "$0")/.." && pwd)"
root="$(cd "$crate_dir/../.." && pwd)"
out="${1:-$crate_dir/pkg}"

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
crate_version="$(sed -n 's/^version = "\(.*\)"$/\1/p' "$crate_dir/Cargo.toml" | head -1)"
npm_version="$(node -p 'require(process.argv[1]).version' "$crate_dir/npm/package.json")"
if [[ "$crate_version" != "$npm_version" ]]; then
    echo "error: npm/package.json version $npm_version != Cargo.toml $crate_version" >&2
    exit 1
fi

(cd "$crate_dir/tools" && npm ci --no-audit --no-fund --loglevel=error)
wasm_opt="$crate_dir/tools/node_modules/.bin/wasm-opt"

target_dir="${CARGO_TARGET_DIR:-$root/target}"
(cd "$root" && cargo build -p openbim-ifc-wasm --target wasm32-unknown-unknown --release)
module="$target_dir/wasm32-unknown-unknown/release/openbim_ifc_wasm.wasm"

rm -rf "$out"
wasm-bindgen --target nodejs --out-dir "$out" "$module"
wasm-bindgen --target bundler --out-dir "$out/bundler" "$module"
wasm-bindgen --target web --out-dir "$out/web" "$module"

# -Oz after wasm-bindgen, which rewrites the module. The --enable flags are
# exactly the features rustc enables by default for wasm32-unknown-unknown
# (Rust >= 1.82): binaryen must accept what the module already uses and
# must not introduce anything newer.
for dir in "$out" "$out/bundler" "$out/web"; do
    wasm="$dir/openbim_ifc_wasm_bg.wasm"
    before="$(wc -c <"$wasm")"
    "$wasm_opt" -Oz \
        --enable-bulk-memory --enable-bulk-memory-opt --enable-multivalue \
        --enable-mutable-globals --enable-nontrapping-float-to-int \
        --enable-reference-types --enable-sign-ext \
        "$wasm" -o "$wasm.opt"
    mv "$wasm.opt" "$wasm"
    echo "wasm-opt -Oz ${dir#"$out"}/: $before -> $(wc -c <"$wasm") bytes"
done

# The nodejs glue is CommonJS. Without its own package.json, Node resolves
# the nearest enclosing one -- inside this repo that is the docs site's, which
# says "type": "module" and breaks loading. The manifest also makes `out` a
# complete npm package. The two ES module targets declare themselves, so
# Node (and any tool that honours "type") reads them as modules even though
# the package root says "type": "commonjs".
cp "$crate_dir/npm/package.json" "$crate_dir/README.md" "$out/"
for dir in bundler web; do
    printf '{\n  "type": "module"\n}\n' >"$out/$dir/package.json"
done

IFC_WASM_PKG="$out" node --test "$crate_dir/tests/js/smoke.mjs" "$crate_dir/tests/js/corpus.mjs"
node "$crate_dir/tools/check-package.mjs" "$out"
