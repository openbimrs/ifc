#!/usr/bin/env bash
# Build the openbim-ifc-capi static library and run the C (and C++) smoke
# test and the cookbook (docs/cookbook/c.md, #331) against the committed
# header. Run from anywhere; used by the gate.
set -euo pipefail

crate_dir="$(cd "$(dirname "$0")/.." && pwd)"
cd "$crate_dir/../.."
target_dir="${CARGO_TARGET_DIR:-$PWD/target}"
cargo build -p openbim-ifc-capi --release

out="$(mktemp -d)"
trap 'rm -rf "$out"' EXIT
lib="$target_dir/release/libopenbim_ifc_capi.a"
# The Rust staticlib needs the platform's threading and dl libraries.
system_libs="-lpthread -ldl -lm"

fixtures="$PWD/test/fixtures"

# build <name> <c11|c++17> <source>: compile one test program into $out.
build() {
    if [[ "$2" == c11 ]]; then
        "${CC:-cc}" -std=c11 -Wall -Wextra -Werror -pedantic \
            -I "$crate_dir/include" "$3" "$lib" $system_libs -o "$out/$1"
    else
        "${CXX:-c++}" -std=c++17 -Wall -Wextra -Werror -x c++ \
            -I "$crate_dir/include" "$3" -x none "$lib" $system_libs -o "$out/$1"
    fi
}

build smoke_c c11 "$crate_dir/tests/c/smoke.c"
"$out/smoke_c"
build cookbook_c c11 "$crate_dir/tests/c/cookbook.c"
"$out/cookbook_c" "$fixtures" "$out"

# The header claims C++ compatibility; prove it compiles and links as C++.
build smoke_cxx c++17 "$crate_dir/tests/c/smoke.c"
"$out/smoke_cxx"
build cookbook_cxx c++17 "$crate_dir/tests/c/cookbook.c"
"$out/cookbook_cxx" "$fixtures" "$out"

# The opt-in mesh build (#328): the same smoke test, now reaching the mesh
# set instead of the `FeatureDisabled` refusal the default library gives,
# and the cookbook's mesh recipe.
cargo build -p openbim-ifc-capi --release --features mesh
build smoke_mesh c11 "$crate_dir/tests/c/smoke.c"
"$out/smoke_mesh"
build cookbook_mesh c11 "$crate_dir/tests/c/cookbook.c"
"$out/cookbook_mesh" "$fixtures" "$out"
