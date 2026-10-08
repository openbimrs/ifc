#!/usr/bin/env bash
# Prove the Debian package of the openbim-ifc command holds what it must
# and installs and removes cleanly (#376). The gate runs it without
# --install wherever dpkg-deb exists; .github/workflows/cli.yml and each
# release build (release.yml, cli-binaries) run it with --install on
# ubuntu-22.04 and ubuntu-22.04-arm.
#
#   check-deb.sh [--install] [--out DIR] [BINARY]
#
# Packs BINARY (default: a fresh debug build) with scripts/package.py into
# openbim-ifc_<version>_<arch>.deb for the Linux target install.sh picks on
# this machine, then:
#   1. `dpkg-deb --info`: the control fields, with no Depends;
#   2. `dpkg-deb --contents`: exactly the binary, README.md and copyright,
#      owned by root, with their modes;
#   3. with --install (needs root or sudo, and dpkg): `dpkg -i`, the
#      installed /usr/bin/openbim-ifc reports the version, `dpkg -r`
#      removes every file and leaves the package not installed.
# --out DIR keeps the .deb in DIR (the release uploads it from there);
# otherwise it is packed into scratch space and deleted.
set -euo pipefail

crate="$(cd "$(dirname "$0")/.." && pwd)"
root="$(cd "$crate/../.." && pwd)"
cd "$root"

install=false
out=""
binary=""
while [[ $# -gt 0 ]]; do
    case "$1" in
        --install) install=true; shift ;;
        --out) out="$2"; shift 2 ;;
        -*) echo "check-deb: unknown option $1" >&2; exit 2 ;;
        *) binary="$1"; shift ;;
    esac
done

fail() { echo "check-deb: $*" >&2; exit 1; }

target_dir="$(cargo metadata --format-version 1 --no-deps |
    python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])')"
if [[ -z "$binary" ]]; then
    cargo build --quiet -p openbim-ifc-cli
    binary="$target_dir/debug/openbim-ifc"
fi
version="$(sed -n 's/^version = "\(.*\)"$/\1/p' "$crate/Cargo.toml" | head -n 1)"
target="$(sh "$crate/install.sh" --print-target)"
case "$target" in
    x86_64-unknown-linux-musl) arch=amd64 ;;
    aarch64-unknown-linux-musl) arch=arm64 ;;
    *) fail "Debian packages are built for Linux only, not $target" ;;
esac

# Scratch space inside the build directory, never /tmp.
mkdir -p "$target_dir"
work="$(mktemp -d "$target_dir/check-deb.XXXXXX")"
trap 'rm -rf "$work"' EXIT
export TMPDIR="$work"
[[ -n "$out" ]] || out="$work/out"

deb="$(python3 "$crate/scripts/package.py" deb --target "$target" --binary "$binary" --out "$out")"
[[ "$(basename "$deb")" == "openbim-ifc_${version}_${arch}.deb" ]] || fail "unexpected name $deb"

# 1. The control file.
dpkg-deb --info "$deb" | tee "$work/info"
field() { dpkg-deb --field "$deb" "$1"; }
[[ "$(field Package)" == openbim-ifc ]] || fail "Package is '$(field Package)'"
[[ "$(field Version)" == "$version" ]] || fail "Version is '$(field Version)', not $version"
[[ "$(field Architecture)" == "$arch" ]] || fail "Architecture is '$(field Architecture)', not $arch"
[[ -n "$(field Maintainer)" ]] || fail "no Maintainer"
[[ -n "$(field Description)" ]] || fail "no Description"
for relation in Depends Pre-Depends Recommends Conflicts; do
    [[ -z "$(field "$relation")" ]] || fail "$relation is set; the binary is static"
done

# 2. The files: exactly these, root-owned, with these modes.
dpkg-deb --contents "$deb" | tee "$work/contents"
awk '$1 !~ /^d/ { print $1, $2, $NF }' "$work/contents" | sort > "$work/files"
sort > "$work/expected" <<'EOF'
-rw-r--r-- root/root ./usr/share/doc/openbim-ifc/README.md
-rw-r--r-- root/root ./usr/share/doc/openbim-ifc/copyright
-rwxr-xr-x root/root ./usr/bin/openbim-ifc
EOF
diff -u "$work/expected" "$work/files" || fail "unexpected files in $deb"
if awk '$1 ~ /^d/ && $2 != "root/root" { found = 1 } END { exit !found }' "$work/contents"; then
    fail "a directory is not owned by root"
fi
# Unpacked to files: a reader stopping early (grep -q) would make tar fail
# on SIGPIPE under pipefail.
mkdir "$work/unpacked"
dpkg-deb --extract "$deb" "$work/unpacked"
cmp -s "$work/unpacked/usr/bin/openbim-ifc" "$binary" || fail "the packaged binary differs from $binary"
grep -q "GNU AFFERO GENERAL PUBLIC LICENSE" "$work/unpacked/usr/share/doc/openbim-ifc/copyright" ||
    fail "copyright lacks the licence text"

# 3. Install, run, remove.
if [[ "$install" == true ]]; then
    sudo=""
    [[ "$(id -u)" == 0 ]] || sudo="sudo"
    if dpkg-query -W -f '${Status}' openbim-ifc 2>/dev/null | grep -q "install ok installed"; then
        fail "openbim-ifc is already installed; remove it first"
    fi
    $sudo dpkg -i "$deb"
    dpkg-query -W -f '${Status}' openbim-ifc | grep -q "install ok installed" || fail "dpkg -i did not install"
    installed="$(/usr/bin/openbim-ifc --version)"
    [[ "$installed" == "openbim-ifc $version" ]] || fail "installed binary says '$installed'"
    /usr/bin/openbim-ifc validate test/fixtures/synthetic-bindings/binding_geometry.ifc >/dev/null ||
        fail "the installed binary could not validate a fixture"
    $sudo dpkg -r openbim-ifc
    for gone in /usr/bin/openbim-ifc /usr/share/doc/openbim-ifc; do
        [[ ! -e "$gone" ]] || fail "dpkg -r left $gone"
    done
    if dpkg-query -W -f '${Status}' openbim-ifc 2>/dev/null | grep -q "ok installed"; then
        fail "openbim-ifc is still installed after dpkg -r"
    fi
    echo "check-deb: installed and removed $(basename "$deb")"
fi

echo "check-deb: ok ($(basename "$deb"))"
