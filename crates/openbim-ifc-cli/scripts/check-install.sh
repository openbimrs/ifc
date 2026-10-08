#!/usr/bin/env bash
# Prove install.sh installs what the release ships, and refuses what it
# must (#329). Runs in the gate on Linux, and on macOS in native.yml and in
# each release build.
#
#   check-install.sh [BINARY]
#
# Packs BINARY (default: a fresh debug build) with scripts/package.py under
# the release name install.sh asks for on this machine, lays it out as a
# release directory with its SHA256SUMS, and serves it to install.sh as a
# file:// mirror. Then:
#   1. install into a prefix; the installed binary reports the version;
#   2. a corrupted SHA256SUMS is refused and installs nothing;
#   3. a version with no release is refused;
#   4. package.py fills the Homebrew formula from a complete SHA256SUMS and
#      refuses an incomplete one.
set -euo pipefail

crate="$(cd "$(dirname "$0")/.." && pwd)"
root="$(cd "$crate/../.." && pwd)"
cd "$root"

binary="${1:-}"
if [[ -z "$binary" ]]; then
    cargo build --quiet -p openbim-ifc-cli
    binary="$(cargo metadata --format-version 1 --no-deps |
        python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])')/debug/openbim-ifc"
fi
version="$(sed -n 's/^version = "\(.*\)"$/\1/p' "$crate/Cargo.toml" | head -n 1)"
target="$(sh "$crate/install.sh" --print-target)"

# Scratch space inside the build directory, never /tmp.
scratch_root="$(cargo metadata --format-version 1 --no-deps |
    python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])')"
mkdir -p "$scratch_root"
work="$(mktemp -d "$scratch_root/check-install.XXXXXX")"
trap 'rm -rf "$work"' EXIT
export TMPDIR="$work"

release="$work/releases/openbim-ifc-cli-v$version"
python3 "$crate/scripts/package.py" archive --target "$target" --binary "$binary" --out "$release" >/dev/null
python3 "$crate/scripts/package.py" checksums "$release" --expect 1 >/dev/null
export OPENBIM_IFC_BASE_URL="file://$release/.."

fail() { echo "check-install: $*" >&2; exit 1; }

# 1. A good install.
sh "$crate/install.sh" --version "$version" --prefix "$work/good"
installed="$("$work/good/bin/openbim-ifc" --version)"
[[ "$installed" == "openbim-ifc $version" ]] || fail "installed binary says '$installed'"
# The tag form of the version is accepted too.
OPENBIM_IFC_PREFIX="$work/tag" sh "$crate/install.sh" --version "openbim-ifc-cli-v$version"
[[ -x "$work/tag/bin/openbim-ifc" ]] || fail "--version openbim-ifc-cli-v$version installed nothing"

# 2. A checksum mismatch installs nothing.
cp "$release/SHA256SUMS" "$work/SHA256SUMS.good"
sed 's/^./0/; s/^0\(.\)/f\1/' "$work/SHA256SUMS.good" > "$release/SHA256SUMS"
cmp -s "$release/SHA256SUMS" "$work/SHA256SUMS.good" && fail "the checksum was not corrupted"
if sh "$crate/install.sh" --version "$version" --prefix "$work/bad" 2>"$work/bad.log"; then
    fail "a corrupted SHA256SUMS was accepted"
fi
grep -q "checksum mismatch" "$work/bad.log" || fail "unexpected refusal: $(cat "$work/bad.log")"
[[ ! -e "$work/bad/bin/openbim-ifc" ]] || fail "a refused download was installed"
cp "$work/SHA256SUMS.good" "$release/SHA256SUMS"

# 3. A version with no release.
if sh "$crate/install.sh" --version 0.0.0-none --prefix "$work/none" 2>/dev/null; then
    fail "a missing release was accepted"
fi

# 4. The Homebrew formula: every platform's checksum, or a refusal.
# Only the checksums matter here, so a stand-in binary keeps it small.
formula_dir="$work/formula"
mkdir -p "$formula_dir"
printf 'stand-in\n' > "$work/stand-in"
for formula_target in aarch64-apple-darwin x86_64-apple-darwin aarch64-unknown-linux-musl x86_64-unknown-linux-musl x86_64-pc-windows-msvc; do
    python3 "$crate/scripts/package.py" archive --target "$formula_target" --binary "$work/stand-in" --out "$formula_dir" >/dev/null
done
python3 "$crate/scripts/package.py" checksums "$formula_dir" --expect 5 >/dev/null
python3 "$crate/scripts/package.py" formula --sums "$formula_dir/SHA256SUMS" --out "$formula_dir/openbim-ifc.rb" >/dev/null
grep -q "version \"$version\"" "$formula_dir/openbim-ifc.rb" || fail "formula lacks the version"
while read -r digest name; do
    [[ "$name" == *.zip ]] && continue
    grep -q "sha256 \"$digest\"" "$formula_dir/openbim-ifc.rb" || fail "formula lacks the checksum of $name"
done < "$formula_dir/SHA256SUMS"
grep -v "x86_64-apple-darwin" "$formula_dir/SHA256SUMS" > "$formula_dir/partial"
if python3 "$crate/scripts/package.py" formula --sums "$formula_dir/partial" >/dev/null 2>&1; then
    fail "a formula was written without every platform's checksum"
fi

echo "check-install: ok ($target, openbim-ifc $version)"
