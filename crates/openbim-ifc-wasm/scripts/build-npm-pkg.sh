#!/usr/bin/env bash
# Build the @openbim/ifc npm package, then test every target it ships.
#
#   crates/openbim-ifc-wasm/scripts/build-npm-pkg.sh [out-dir]
#
Two release builds of the wasm module, each bound three times by
# wasm-bindgen:
#
#   <out>/            --target nodejs   CommonJS for Node (the `node` condition)
#   <out>/bundler/    --target bundler  ES module for webpack, Vite, Rollup
#   <out>/web/        --target web      ES module for a browser, no bundler
#
# from the default features (`@openbim/ifc`), and the same three under
# <out>/mesh/ from the default features plus `mesh` (`@openbim/ifc/mesh`,
# #369), whose `productMeshes` links the reference geometry backend
# (ADR 0021). A consumer who never imports the mesh entry never downloads
# it into a bundle or a page; it costs only package size.
#
# The module embeds no PSD/QTO catalog (#318). The package carries one
# snapshot file per edition in <out>/catalog/, written from the committed
# container by ifc-template-catalog's export_snapshots example, which checks
# each against the SHA-256 the module pins; and npm/catalog.mjs, the loader
# behind `IfcModel.loadCatalog`, installed on each target's IfcModel below.
#
# Then the Node smoke, corpus, geometry and cookbook suites run against
# <out> (the geometry suite against <out>/mesh too; the cookbook's mesh
# recipe reads <out>/mesh itself), and
# tools/check-package.mjs packs <out> as npm would publish it and checks each
# target of both entries from that tarball: Node `require` and `import`, a
# webpack bundle, and both browser builds in headless Chrome (#40, #369).
#
# The wasm-bindgen CLI must match the `wasm-bindgen` crate version pinned in
# Cargo.toml exactly: a mismatch fails at bindgen time with a schema error,
# or worse, generates glue for a different ABI. This script refuses early and
# names the version to install. webpack comes pinned from
# tools/package-lock.json.
#
# IFC_NPM_BUILD_ONLY=1 stops after the package is assembled, before any
# suite: the Pages workflow builds the package for the docs site's API
# reference and playground (#331), and the gate has already tested it.
#
# No wasm-opt pass: measured with binaryen 132 on this module (#40), -Oz cut
# the raw size 3.1% but grew it 0.5% under gzip -9 and 0.7% under brotli,
# which is what a browser downloads.
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

if [[ -z "${IFC_NPM_BUILD_ONLY:-}" ]]; then
    (cd "$crate_dir/tools" && npm ci --no-audit --no-fund --loglevel=error)
fi

target_dir="${CARGO_TARGET_DIR:-$root/target}"
module="$target_dir/wasm32-unknown-unknown/release/openbim_ifc_wasm.wasm"

# bind <dir> <loader> <es-loader>: bind $module for the three targets into
# <dir>, <dir>/bundler and <dir>/web, and install `IfcModel.loadCatalog` on
# each. <loader> is the package root's catalog.mjs relative to <dir>,
# <es-loader> relative to <dir>/bundler and <dir>/web.
bind() {
    local dir="$1" loader="$2" es_loader="$3"
    wasm-bindgen --target nodejs --out-dir "$dir" "$module"
    wasm-bindgen --target bundler --out-dir "$dir/bundler" "$module"
    wasm-bindgen --target web --out-dir "$dir/web" "$module"

    # The two ES module targets declare themselves, so Node (and any tool
    # that honours "type") reads them as modules even though the package
    # root says "type": "commonjs".
    for target in bundler web; do
        printf '{\n  "type": "module"\n}\n' >"$dir/$target/package.json"
    done

    # `IfcModel.loadCatalog` on each target (#318). The CommonJS glue cannot
    # import an ES module synchronously, so it imports the loader on first
    # use; the two ES module targets import it statically. The bundler glue
    # re-exports the class from its `_bg.js` module rather than defining it.
    cat >>"$dir/openbim_ifc_wasm.js" <<JS

// PSD/QTO catalog loader (#318); appended by scripts/build-npm-pkg.sh.
{
    let loader;
    IfcModel.loadCatalog = function loadCatalog(release, options) {
        loader ??= import("${loader}").then((m) => m.catalogLoader(IfcModel));
        return loader.then((load) => load(release, options));
    };
}
JS
    cat >>"$dir/bundler/openbim_ifc_wasm.js" <<JS

// PSD/QTO catalog loader (#318); appended by scripts/build-npm-pkg.sh.
import { IfcModel as __IfcModel } from "./openbim_ifc_wasm_bg.js";
import { catalogLoader as __catalogLoader } from "${es_loader}";
__IfcModel.loadCatalog = __catalogLoader(__IfcModel);
JS
    cat >>"$dir/web/openbim_ifc_wasm.js" <<JS

// PSD/QTO catalog loader (#318); appended by scripts/build-npm-pkg.sh.
import { catalogLoader as __catalogLoader } from "${es_loader}";
IfcModel.loadCatalog = __catalogLoader(IfcModel);
JS
}

rm -rf "$out"

# The default entry: every default feature, no meshes.
(cd "$root" && cargo build -p openbim-ifc-wasm --target wasm32-unknown-unknown --release)
bind "$out" ./catalog.mjs ../catalog.mjs

# The mesh entry (#369): the same features plus `mesh`, which links the
# reference geometry backend (ADR 0021). Built after the default entry is
# bound, since both builds write the same module path.
(cd "$root" && cargo build -p openbim-ifc-wasm --target wasm32-unknown-unknown --release --features mesh)
bind "$out/mesh" ../catalog.mjs ../../catalog.mjs

# The nodejs glue is CommonJS. Without its own package.json, Node resolves
# the nearest enclosing one -- inside this repo that is the docs site's, which
# says "type": "module" and breaks loading. The manifest also makes `out` a
# complete npm package; its "type": "commonjs" covers mesh/ too.
cp "$crate_dir/npm/package.json" "$crate_dir/npm/catalog.mjs" "$crate_dir/README.md" "$out/"

# The catalog snapshots, one per edition (#318), shared by both entries.
(cd "$root" && cargo run --quiet --release -p ifc-template-catalog --features runtime \
    --example export_snapshots -- "$out/catalog")

if [[ -n "${IFC_NPM_BUILD_ONLY:-}" ]]; then
    echo "built $out (IFC_NPM_BUILD_ONLY set; suites NOT run)"
    exit 0
fi

IFC_WASM_PKG="$out" node --test "$crate_dir/tests/js/smoke.mjs" "$crate_dir/tests/js/corpus.mjs" \
    "$crate_dir/tests/js/geometry.mjs" "$crate_dir/tests/js/cookbook.mjs"
# The mesh entry's meshes, refusals and the viewer example's scene code; the
# packaged smoke below covers the rest of its surface and its catalog.
IFC_WASM_PKG="$out/mesh" IFC_WASM_MESH=1 node --test "$crate_dir/tests/js/geometry.mjs"
node "$crate_dir/tools/check-package.mjs" "$out"
