#!/usr/bin/env python3
"""Pack the OpenBim.Ifc NuGet package and prove a fresh project can use it (#327).

The package is P/Invoke over the C ABI (openbim-ifc-capi) with the C ABI's
shared library inside, one per runtime identifier under
`runtimes/<rid>/native/`. This script:

1. stages those libraries: the host's, built here with cargo (default), or
   every one from the release archives of openbim-ifc-capi (`--archives`);
2. packs the `.nupkg` with `dotnet pack` and checks its layout: both
   assemblies (netstandard2.0, net8.0) with their XML docs, the .NET
   Framework build targets, the README and licence, and each staged library;
3. copies the C# test suite to a fresh directory, outside the source tree,
   points it at a local feed holding only that `.nupkg` (the package can
   come from nowhere else), restores into an empty package cache and runs
   it -- on .NET 8, and on Windows x64 also on .NET Framework 4.8.

So the suite tests what `dotnet add package OpenBim.Ifc` installs, the way
crates/openbim-ifc-wasm/tools/check-package.mjs tests the npm tarball.

Usage:
    check-dotnet.py                          # build, pack, test (gate, CI)
    check-dotnet.py --archives DIR --out DIR --rids all
                                             # release: pack every RID, no test
    check-dotnet.py --test-only --feed DIR   # release: test a packed .nupkg
"""

from __future__ import annotations

import argparse
import os
import platform
import re
import shutil
import subprocess
import sys
import tarfile
import zipfile
from pathlib import Path

CRATE = Path(__file__).resolve().parents[1]
ROOT = CRATE.parents[1]
PROJECT = CRATE / "dotnet" / "OpenBim.Ifc" / "OpenBim.Ifc.csproj"
TESTS = CRATE / "dotnet" / "OpenBim.Ifc.Tests"
PACKAGE = "OpenBim.Ifc"

# Rust target of each release archive -> .NET runtime identifier.
RIDS = {
    "x86_64-unknown-linux-gnu": "linux-x64",
    "aarch64-unknown-linux-gnu": "linux-arm64",
    "x86_64-apple-darwin": "osx-x64",
    "aarch64-apple-darwin": "osx-arm64",
    "x86_64-pc-windows-msvc": "win-x64",
    "aarch64-pc-windows-msvc": "win-arm64",
}


def library_file(rid: str) -> str:
    if rid.startswith("win-"):
        return "openbim_ifc_capi.dll"
    if rid.startswith("osx-"):
        return "libopenbim_ifc_capi.dylib"
    return "libopenbim_ifc_capi.so"


def archive_member(rid: str) -> str:
    """Where the shared library sits in an openbim-ifc-capi archive."""
    return ("bin/" if rid.startswith("win-") else "lib/") + library_file(rid)


def run(*args: object, cwd: Path | None = None, env: dict[str, str] | None = None) -> None:
    command = [str(a) for a in args]
    print("+ " + " ".join(command), flush=True)
    subprocess.run(command, cwd=cwd, env=env, check=True)


def version_of(path: Path, pattern: str) -> str:
    found = re.search(pattern, path.read_text(encoding="utf-8"), re.MULTILINE)
    if not found:
        raise SystemExit(f"no version in {path}")
    return found.group(1)


def package_version() -> str:
    """The .csproj version, which must equal the crate's (one tag, one version)."""
    crate = version_of(CRATE / "Cargo.toml", r'^version = "([^"]+)"')
    csproj = version_of(PROJECT, r"<Version>([^<]+)</Version>")
    if crate != csproj:
        raise SystemExit(f"{PROJECT.relative_to(ROOT)} says {csproj} but Cargo.toml says {crate}")
    return crate


def host_rid() -> str:
    machine = platform.machine().lower()
    arch = {"x86_64": "x64", "amd64": "x64", "arm64": "arm64", "aarch64": "arm64"}.get(machine)
    system = {"Linux": "linux", "Darwin": "osx", "Windows": "win"}.get(platform.system())
    if arch is None or system is None:
        raise SystemExit(f"no OpenBim.Ifc runtime for {platform.system()} {machine}")
    return f"{system}-{arch}"


def cargo_target_dir() -> Path:
    env = os.environ.get("CARGO_TARGET_DIR")
    return Path(env) if env else ROOT / "target"


def stage_host(native: Path) -> list[str]:
    """Build the C ABI for this machine and stage its shared library."""
    run("cargo", "build", "-p", "openbim-ifc-capi", "--release", "--locked", cwd=ROOT)
    rid = host_rid()
    built = cargo_target_dir() / "release" / library_file(rid)
    (native / rid).mkdir(parents=True)
    shutil.copy2(built, native / rid / library_file(rid))
    return [rid]


def stage_archives(native: Path, archives: Path) -> list[str]:
    """Stage the shared library of every release archive in `archives`."""
    rids = []
    for archive in sorted(archives.iterdir()):
        name = archive.name
        if not name.startswith("openbim-ifc-capi-v") or not name.endswith((".tar.gz", ".zip")):
            continue
        target = next((t for t in RIDS if re.search(rf"-{re.escape(t)}\.(tar\.gz|zip)$", name)), None)
        if target is None:
            raise SystemExit(f"{name}: no runtime identifier for its target")
        rid = RIDS[target]
        stem = name[: -len(".tar.gz")] if name.endswith(".tar.gz") else name[: -len(".zip")]
        member = f"{stem}/{archive_member(rid)}"
        destination = native / rid / library_file(rid)
        destination.parent.mkdir(parents=True)
        if name.endswith(".zip"):
            with zipfile.ZipFile(archive) as zf, zf.open(member) as source:
                destination.write_bytes(source.read())
        else:
            with tarfile.open(archive) as tf:
                source = tf.extractfile(member)
                if source is None:
                    raise SystemExit(f"{name}: {member} is not a file")
                destination.write_bytes(source.read())
        rids.append(rid)
        print(f"staged {rid} from {name}")
    if not rids:
        raise SystemExit(f"no openbim-ifc-capi archive in {archives}")
    return rids


def pack(native: Path, out: Path, version: str, rids: list[str]) -> Path:
    out.mkdir(parents=True, exist_ok=True)
    run("dotnet", "pack", PROJECT, "-c", "Release", "-o", out,
        f"-p:NativeDir={native}", "-p:ContinuousIntegrationBuild=true")
    nupkg = out / f"{PACKAGE}.{version}.nupkg"
    if not nupkg.is_file():
        raise SystemExit(f"dotnet pack did not write {nupkg}")
    expected = {
        "lib/netstandard2.0/OpenBim.Ifc.dll",
        "lib/netstandard2.0/OpenBim.Ifc.xml",
        "lib/net8.0/OpenBim.Ifc.dll",
        "lib/net8.0/OpenBim.Ifc.xml",
        "build/net462/OpenBim.Ifc.targets",
        "buildTransitive/net462/OpenBim.Ifc.targets",
        "README.md",
        "LICENSE",
        "NOTICE",
        *(f"runtimes/{rid}/native/{library_file(rid)}" for rid in rids),
    }
    with zipfile.ZipFile(nupkg) as zf:
        names = set(zf.namelist())
    missing = sorted(expected - names)
    if missing:
        raise SystemExit(f"{nupkg.name} lacks {missing}")
    stray = sorted(n for n in names if n.startswith("runtimes/") and n not in expected)
    if stray:
        raise SystemExit(f"{nupkg.name} carries unexpected native files {stray}")
    print(f"packed {nupkg} with {', '.join(sorted(rids))}")
    return nupkg


NUGET_CONFIG = """<?xml version="1.0" encoding="utf-8"?>
<configuration>
  <packageSources>
    <clear />
    <add key="local" value="{feed}" />
    <add key="nuget.org" value="https://api.nuget.org/v3/index.json" />
  </packageSources>
  <!-- OpenBim.Ifc only from the local feed, even once nuget.org has one. -->
  <packageSourceMapping>
    <packageSource key="local">
      <package pattern="OpenBim.Ifc" />
    </packageSource>
    <packageSource key="nuget.org">
      <package pattern="*" />
    </packageSource>
  </packageSourceMapping>
</configuration>
"""


def test(feed: Path, work: Path, version: str) -> None:
    """Run the C# suite in a fresh project against the package in `feed`."""
    consumer = work / "consumer"
    shutil.rmtree(consumer, ignore_errors=True)
    consumer.mkdir(parents=True)
    for source in sorted(TESTS.iterdir()):
        if source.suffix in (".cs", ".csproj"):
            shutil.copy2(source, consumer / source.name)
    # Stop MSBuild's upward search for repository build settings.
    for name in ("Directory.Build.props", "Directory.Build.targets", "Directory.Packages.props"):
        (consumer / name).write_text("<Project />\n", encoding="utf-8")
    (consumer / "nuget.config").write_text(NUGET_CONFIG.format(feed=feed.resolve()), encoding="utf-8")
    # An empty cache: a package of the same version restored earlier must
    # not stand in for the one just packed.
    packages = work / "packages"
    shutil.rmtree(packages, ignore_errors=True)
    run("dotnet", "test", consumer / "OpenBim.Ifc.Tests.csproj", "-c", "Release",
        f"-p:RestorePackagesPath={packages}", f"-p:OpenBimIfcVersion={version}",
        "--logger", "console;verbosity=normal")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--archives", type=Path,
                        help="stage every RID from these openbim-ifc-capi release archives")
    parser.add_argument("--out", type=Path, help="write the .nupkg here (default: the work dir)")
    parser.add_argument("--rids", help="comma-separated runtime identifiers the package must "
                        "carry, exactly; `all` for every one the release ships")
    parser.add_argument("--pack-only", action="store_true", help="pack, do not test")
    parser.add_argument("--test-only", action="store_true",
                        help="test the .nupkg in --feed instead of packing one")
    parser.add_argument("--feed", type=Path, help="with --test-only: the directory holding the .nupkg")
    parser.add_argument("--work", type=Path, help="scratch directory (default: <target>/dotnet-check)")
    args = parser.parse_args()

    os.environ.setdefault("DOTNET_CLI_TELEMETRY_OPTOUT", "1")
    os.environ.setdefault("DOTNET_NOLOGO", "1")
    if shutil.which("dotnet") is None:
        raise SystemExit("the .NET 8 SDK is needed (dotnet not on PATH)")
    version = package_version()
    work = (args.work or cargo_target_dir() / "dotnet-check").resolve()

    if args.test_only:
        if args.feed is None:
            raise SystemExit("--test-only needs --feed")
        test(args.feed.resolve(), work, version)
        return 0

    native = work / "native"
    shutil.rmtree(native, ignore_errors=True)
    native.mkdir(parents=True)
    rids = stage_archives(native, args.archives.resolve()) if args.archives else stage_host(native)
    if args.rids:
        required = sorted(RIDS.values()) if args.rids == "all" else sorted(args.rids.split(","))
        if sorted(rids) != required:
            raise SystemExit(f"staged {sorted(rids)}, but the package must carry {required}")
    feed = (args.out or work / "feed").resolve()
    shutil.rmtree(feed, ignore_errors=True)
    pack(native, feed, version, rids)
    if not args.pack_only and not args.archives:
        test(feed, work, version)
    return 0


if __name__ == "__main__":
    sys.exit(main())
