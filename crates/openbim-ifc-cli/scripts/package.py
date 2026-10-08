#!/usr/bin/env python3
"""Pack the openbim-ifc binary the way its GitHub release ships it (#329).

Four steps, each used by the release workflow and by the check scripts
(check-install.sh, check-install.ps1, check-deb.sh), so what the installers
are tested against is what the release attaches:

    package.py archive --target T --binary PATH --out DIR
        DIR/openbim-ifc-v<version>-<T>.tar.gz (a .zip for a Windows
        target), holding openbim-ifc-v<version>-<T>/ with the binary,
        README.md and LICENSE.
    package.py deb --target T --binary PATH --out DIR
        DIR/openbim-ifc_<version>_<arch>.deb for a Linux target (#376):
        the binary in /usr/bin, README.md and the licence in
        /usr/share/doc/openbim-ifc/, no Depends (the binary is static).
        Needs dpkg-deb.
    package.py checksums DIR [--expect N]
        DIR/SHA256SUMS over every archive and .deb in DIR, in `sha256sum`
        format; with --expect, refuses unless there are exactly N.
    package.py formula --sums DIR/SHA256SUMS [--out FILE]
        The Homebrew formula for this version from
        packaging/openbim-ifc.rb.in, with each platform's checksum.
        Refuses when an archive the formula names is missing.

The version is the crate's, read from its Cargo.toml.
"""

from __future__ import annotations

import argparse
import hashlib
import io
import os
import re
import shutil
import subprocess
import sys
import tarfile
import tempfile
import zipfile
from pathlib import Path

CRATE = Path(__file__).resolve().parents[1]
ROOT = CRATE.parents[1]
BIN = "openbim-ifc"
TEMPLATE = CRATE / "packaging" / "openbim-ifc.rb.in"
# The targets the formula installs from: Linux as static musl binaries.
FORMULA_TARGETS = (
    "aarch64-apple-darwin",
    "x86_64-apple-darwin",
    "aarch64-unknown-linux-musl",
    "x86_64-unknown-linux-musl",
)
# Debian architecture of each Linux target the release builds a .deb from.
DEB_ARCH = {
    "x86_64-unknown-linux-musl": "amd64",
    "aarch64-unknown-linux-musl": "arm64",
}
DEB_PACKAGE = "openbim-ifc"
DEB_MAINTAINER = "OpenBIM.rs <https://github.com/openbimrs/ifc/issues>"
HOMEPAGE = "https://openbimrs.github.io/ifc/guide/cli"


def version() -> str:
    text = (CRATE / "Cargo.toml").read_text(encoding="utf-8")
    found = re.search(r'^version = "([^"]+)"', text, re.MULTILINE)
    if not found:
        raise SystemExit("no version in Cargo.toml")
    return found.group(1)


def stem(target: str) -> str:
    return f"{BIN}-v{version()}-{target}"


def archive(target: str, binary: Path, out: Path) -> Path:
    windows = "windows" in target
    name = BIN + (".exe" if windows else "")
    if not binary.is_file():
        raise SystemExit(f"no binary at {binary}")
    folder = stem(target)
    members = [
        (f"{folder}/{name}", binary.read_bytes(), 0o755),
        (f"{folder}/README.md", (CRATE / "README.md").read_bytes(), 0o644),
        (f"{folder}/LICENSE", (ROOT / "LICENSE").read_bytes(), 0o644),
    ]
    out.mkdir(parents=True, exist_ok=True)
    if windows:
        path = out / f"{folder}.zip"
        with zipfile.ZipFile(path, "w", zipfile.ZIP_DEFLATED) as packed:
            for member, data, mode in members:
                info = zipfile.ZipInfo(member, date_time=(1980, 1, 1, 0, 0, 0))
                info.external_attr = (0o100000 | mode) << 16
                info.compress_type = zipfile.ZIP_DEFLATED
                packed.writestr(info, data)
    else:
        path = out / f"{folder}.tar.gz"
        with tarfile.open(path, "w:gz") as packed:
            for member, data, mode in members:
                info = tarfile.TarInfo(member)
                info.size = len(data)
                info.mode = mode
                packed.addfile(info, io.BytesIO(data))
    print(path)
    return path


def deb_name(target: str) -> str:
    if target not in DEB_ARCH:
        known = ", ".join(DEB_ARCH)
        raise SystemExit(f"no Debian architecture for {target}; known: {known}")
    return f"{DEB_PACKAGE}_{version()}_{DEB_ARCH[target]}.deb"


def deb_copyright() -> str:
    """debian/copyright in the machine-readable format, licence in full."""
    licence = (ROOT / "LICENSE").read_text(encoding="utf-8")
    body = "".join(
        f" {line}\n" if line.strip() else " .\n" for line in licence.rstrip().splitlines()
    )
    return (
        "Format: https://www.debian.org/doc/packaging-manuals/copyright-format/1.0/\n"
        f"Upstream-Name: {DEB_PACKAGE}\n"
        "Source: https://github.com/openbimrs/ifc\n"
        "\n"
        "Files: *\n"
        "Copyright: point-grey and the openbimrs/ifc contributors\n"
        "License: AGPL-3.0-or-later\n"
        f"{body}"
    )


def deb(target: str, binary: Path, out: Path) -> Path:
    if not binary.is_file():
        raise SystemExit(f"no binary at {binary}")
    if shutil.which("dpkg-deb") is None:
        raise SystemExit("dpkg-deb is needed to build a .deb")
    name = deb_name(target)
    # Every file and directory gets a fixed time, so the same binary packs
    # to the same .deb (dpkg-deb also reads SOURCE_DATE_EPOCH).
    epoch = int(os.environ.get("SOURCE_DATE_EPOCH", "315532800"))
    out.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="deb.", dir=out) as staging:
        tree = Path(staging) / "root"
        doc = tree / "usr" / "share" / "doc" / DEB_PACKAGE
        files = {
            tree / "usr" / "bin" / BIN: (binary.read_bytes(), 0o755),
            doc / "README.md": ((CRATE / "README.md").read_bytes(), 0o644),
            doc / "copyright": (deb_copyright().encode("utf-8"), 0o644),
        }
        for path, (data, mode) in files.items():
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(data)
            path.chmod(mode)
        size_kib = sum((len(data) + 1023) // 1024 for data, _ in files.values())
        md5sums = "".join(
            f"{hashlib.md5(data).hexdigest()}  {path.relative_to(tree).as_posix()}\n"
            for path, (data, _) in sorted(files.items())
        )
        control = (
            f"Package: {DEB_PACKAGE}\n"
            f"Version: {version()}\n"
            f"Architecture: {DEB_ARCH[target]}\n"
            f"Maintainer: {DEB_MAINTAINER}\n"
            f"Installed-Size: {size_kib}\n"
            "Section: science\n"
            "Priority: optional\n"
            f"Homepage: {HOMEPAGE}\n"
            "Description: validate, convert and inspect IFC files\n"
            " The openbim-ifc command checks IFC models (STEP and ifcXML) against the\n"
            " schema they declare, converts between STEP and ifcXML, and reports\n"
            " headers, property sets, the spatial tree and unreachable products,\n"
            " with exit codes and JSON or SARIF output for scripts and CI.\n"
            " .\n"
            " A static binary built from the openbim-ifc-cli crate of openbimrs/ifc.\n"
        )
        meta = tree / "DEBIAN"
        meta.mkdir()
        (meta / "control").write_text(control, encoding="utf-8")
        (meta / "md5sums").write_text(md5sums, encoding="utf-8")
        for path in [tree, *tree.rglob("*")]:
            if path.is_dir():
                path.chmod(0o755)
            elif path.parent == meta:
                path.chmod(0o644)
        for path in [tree, *tree.rglob("*")]:
            os.utime(path, (epoch, epoch), follow_symlinks=False)
        target_path = out / name
        env = {**os.environ, "SOURCE_DATE_EPOCH": str(epoch)}
        subprocess.run(
            ["dpkg-deb", "--root-owner-group", "-Zxz", "--build", str(tree), str(target_path)],
            check=True, env=env, stdout=subprocess.DEVNULL,
        )
    print(target_path)
    return target_path


def is_release_file(path: Path) -> bool:
    name = path.name
    return (
        name.startswith(f"{BIN}-v") and name.endswith((".tar.gz", ".zip"))
    ) or (name.startswith(f"{DEB_PACKAGE}_") and name.endswith(".deb"))


def checksums(directory: Path, expect: int | None) -> Path:
    archives = sorted(path for path in directory.iterdir() if is_release_file(path))
    if expect is not None and len(archives) != expect:
        names = ", ".join(path.name for path in archives) or "none"
        raise SystemExit(f"expected {expect} archives and packages, found {len(archives)}: {names}")
    lines = [
        f"{hashlib.sha256(path.read_bytes()).hexdigest()}  {path.name}\n" for path in archives
    ]
    sums = directory / "SHA256SUMS"
    sums.write_text("".join(lines), encoding="utf-8")
    sys.stdout.write("".join(lines))
    return sums


def formula(sums: Path) -> str:
    listed = {}
    for line in sums.read_text(encoding="utf-8").splitlines():
        digest, _, name = line.partition("  ")
        listed[name.lstrip("*")] = digest
    text = TEMPLATE.read_text(encoding="utf-8").replace("@VERSION@", version())
    for target in FORMULA_TARGETS:
        name = f"{stem(target)}.tar.gz"
        if name not in listed:
            raise SystemExit(f"{sums} lists no {name}")
        text = text.replace(f"@SHA256:{target}@", listed[name])
    left = re.findall(r"@[A-Z0-9_:.a-z-]+@", text)
    if left:
        raise SystemExit(f"unfilled placeholders in the formula: {left}")
    return text


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__,
        formatter_class=argparse.RawDescriptionHelpFormatter)
    steps = parser.add_subparsers(dest="step", required=True)
    packed = steps.add_parser("archive")
    packed.add_argument("--target", required=True)
    packed.add_argument("--binary", required=True, type=Path)
    packed.add_argument("--out", required=True, type=Path)
    debbed = steps.add_parser("deb")
    debbed.add_argument("--target", required=True, choices=sorted(DEB_ARCH))
    debbed.add_argument("--binary", required=True, type=Path)
    debbed.add_argument("--out", required=True, type=Path)
    summed = steps.add_parser("checksums")
    summed.add_argument("directory", type=Path)
    summed.add_argument("--expect", type=int)
    brewed = steps.add_parser("formula")
    brewed.add_argument("--sums", required=True, type=Path)
    brewed.add_argument("--out", type=Path)
    args = parser.parse_args()

    if args.step == "archive":
        archive(args.target, args.binary, args.out)
    elif args.step == "deb":
        deb(args.target, args.binary, args.out)
    elif args.step == "checksums":
        checksums(args.directory, args.expect)
    else:
        text = formula(args.sums)
        if args.out:
            args.out.write_text(text, encoding="utf-8")
            print(args.out)
        else:
            sys.stdout.write(text)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
