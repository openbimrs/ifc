#!/usr/bin/env bash
# Build the openbim-ifc-capi static library and run the C (and C++) smoke
# test against the committed header. Run from anywhere; used by the gate.
set -euo pipefail

crate_dir="$(cd "$(dirname "$0")/.." && pwd)"
cd "$crate_dir/.."
target_dir="${CARGO_TARGET_DIR:-$PWD/target}"
cargo build -p openbim-ifc-capi --release

out="$(mktemp -d)"
trap 'rm -rf "$out"' EXIT
lib="$target_dir/release/libopenbim_ifc_capi.a"
# The Rust staticlib needs the platform's threading and dl libraries.
system_libs="-lpthread -ldl -lm"

"${CC:-cc}" -std=c11 -Wall -Wextra -Werror -pedantic \
    -I "$crate_dir/include" "$crate_dir/tests/c/smoke.c" \
    "$lib" $system_libs -o "$out/smoke_c"
"$out/smoke_c"

# The header claims C++ compatibility; prove it compiles and links as C++.
"${CXX:-c++}" -std=c++17 -Wall -Wextra -Werror -x c++ \
    -I "$crate_dir/include" "$crate_dir/tests/c/smoke.c" \
    -x none "$lib" $system_libs -o "$out/smoke_cxx"
"$out/smoke_cxx"
