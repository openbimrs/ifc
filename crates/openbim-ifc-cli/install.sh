#!/bin/sh
# Install the openbim-ifc command from a GitHub release of openbimrs/ifc.
#
#   curl -fsSL https://raw.githubusercontent.com/openbimrs/ifc/main/crates/openbim-ifc-cli/install.sh | sh
#   curl -fsSL .../install.sh | sh -s -- --version 0.1.0 --prefix /usr/local
#
# It picks the archive for this OS and CPU (Linux x86_64 and aarch64, as
# static musl binaries; macOS x86_64 and arm64), downloads it with the
# release's SHA256SUMS, refuses to install unless the checksum matches, and
# copies the binary to PREFIX/bin (default ~/.local/bin). Windows: use
# install.ps1 beside this script.
#
# Options (or the environment variable in brackets):
#   --version V   a release, e.g. 0.1.0; default the newest  [OPENBIM_IFC_VERSION]
#   --prefix DIR  install into DIR/bin; default ~/.local      [OPENBIM_IFC_PREFIX]
#   --print-target  print the release target for this machine and exit
# Mirrors and tests may point the downloads elsewhere: OPENBIM_IFC_BASE_URL
# (default https://github.com/openbimrs/ifc/releases/download), under which
# each release is a directory named after its tag. Needs curl or wget, tar,
# and sha256sum or shasum.
set -eu

repo="openbimrs/ifc"
crate="openbim-ifc-cli"
bin="openbim-ifc"
version="${OPENBIM_IFC_VERSION:-}"
prefix="${OPENBIM_IFC_PREFIX:-${HOME:-}/.local}"
base="${OPENBIM_IFC_BASE_URL:-https://github.com/$repo/releases/download}"
print_target=false

say() { printf 'install.sh: %s\n' "$*" >&2; }
fail() { say "error: $*"; exit 1; }

while [ $# -gt 0 ]; do
    case "$1" in
        --version) [ $# -ge 2 ] || fail "--version needs a value"; version="$2"; shift 2 ;;
        --version=*) version="${1#*=}"; shift ;;
        --prefix) [ $# -ge 2 ] || fail "--prefix needs a value"; prefix="$2"; shift 2 ;;
        --prefix=*) prefix="${1#*=}"; shift ;;
        --print-target) print_target=true; shift ;;
        -h | --help) sed -n '2,23p' "$0" 2>/dev/null || true; exit 0 ;;
        *) fail "unknown option $1 (see --help)" ;;
    esac
done

# The release target for this machine: the names release.yml builds.
detect_target() {
    os="$(uname -s)"
    arch="$(uname -m)"
    case "$arch" in
        x86_64 | amd64) arch=x86_64 ;;
        aarch64 | arm64) arch=aarch64 ;;
        *) fail "no prebuilt binary for CPU $arch; build it with: cargo install $crate" ;;
    esac
    case "$os" in
        Linux) echo "$arch-unknown-linux-musl" ;;
        Darwin)
            # An x86_64 shell under Rosetta on Apple silicon still gets the
            # native arm64 binary.
            if [ "$arch" = x86_64 ] && [ "$(sysctl -n hw.optional.arm64 2>/dev/null || echo 0)" = 1 ]; then
                arch=aarch64
            fi
            echo "$arch-apple-darwin"
            ;;
        *) fail "no installer for $os; download the archive from https://github.com/$repo/releases" ;;
    esac
}

target="$(detect_target)"
if [ "$print_target" = true ]; then
    echo "$target"
    exit 0
fi

download() { # url destination
    if command -v curl >/dev/null 2>&1; then
        curl --proto '=https,file' --tlsv1.2 -fsSL "$1" -o "$2"
    elif command -v wget >/dev/null 2>&1; then
        wget -q -O "$2" "$1"
    else
        fail "needs curl or wget"
    fi
}

sha256() { # file
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$1" | cut -d ' ' -f 1
    elif command -v shasum >/dev/null 2>&1; then
        shasum -a 256 "$1" | cut -d ' ' -f 1
    else
        fail "needs sha256sum or shasum to verify the download"
    fi
}

# The newest openbim-ifc-cli release: other crates of the repository
# release under their own tags, so "latest" is not necessarily this one.
if [ -z "$version" ]; then
    api="https://api.github.com/repos/$repo/releases?per_page=100"
    listing="$(mktemp)"
    download "$api" "$listing" || fail "could not list the releases of $repo; pass --version"
    version="$(grep -o "\"tag_name\": *\"$crate-v[^\"]*\"" "$listing" | head -n 1 | sed 's/.*-v\([^"]*\)"/\1/')"
    rm -f "$listing"
    [ -n "$version" ] || fail "no $crate release found; pass --version"
fi
version="${version#"$crate"-}"
version="${version#v}"

tag="$crate-v$version"
stem="$bin-v$version-$target"
archive="$stem.tar.gz"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
trap 'exit 1' INT TERM

say "downloading $archive ($tag)"
download "$base/$tag/$archive" "$work/$archive" || fail "could not download $base/$tag/$archive"
download "$base/$tag/SHA256SUMS" "$work/SHA256SUMS" || fail "could not download $base/$tag/SHA256SUMS"

expected="$(awk -v name="$archive" '$2 == name || $2 == "*" name { print $1 }' "$work/SHA256SUMS")"
[ -n "$expected" ] || fail "SHA256SUMS lists no $archive"
actual="$(sha256 "$work/$archive")"
[ "$expected" = "$actual" ] || fail "checksum mismatch for $archive: expected $expected, got $actual; nothing installed"

tar -xzf "$work/$archive" -C "$work"
[ -f "$work/$stem/$bin" ] || fail "$archive holds no $stem/$bin"
mkdir -p "$prefix/bin"
cp "$work/$stem/$bin" "$prefix/bin/$bin.tmp"
chmod 755 "$prefix/bin/$bin.tmp"
mv -f "$prefix/bin/$bin.tmp" "$prefix/bin/$bin"
say "installed $("$prefix/bin/$bin" --version) to $prefix/bin/$bin"

case ":${PATH:-}:" in
    *":$prefix/bin:"*) ;;
    *) say "$prefix/bin is not on PATH; add it, e.g.: export PATH=\"$prefix/bin:\$PATH\"" ;;
esac
