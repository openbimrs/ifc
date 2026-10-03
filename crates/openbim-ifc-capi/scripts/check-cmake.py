#!/usr/bin/env python3
"""Build the openbim_ifc CMake package and prove a consumer can use it (#41),
through CMake and through pkg-config (#325).

Runs on Linux, macOS and Windows (CI) and in the gate:

1. Source tree: a consumer that `add_subdirectory`s the crate, configured
   while the shared library does not exist yet. This is the fresh-build case
   in which macOS used to link without an LC_RPATH (axiolid/kernel#113).
2. Package: `cmake --install` the crate into a prefix and pack that prefix
   into the release archive (`--archive-dir` keeps it). The archive is then
   unpacked elsewhere, and every consumer below uses the unpacked copy, so a
   path baked in at install time would fail.
3. SHARED and STATIC consumers via `find_package(openbim_ifc)`, each running
   tests/c/smoke.c as C11 and C++17 through ctest. The STATIC consumer builds
   against a copy with the shared library deleted; the SHARED one must fail
   to start once its library is moved, and start again when the platform's
   library search path names the new place. Each proves it links what it
   claims to, and the second that the library is relocatable.
4. pkg-config, on Linux and macOS, against the installed tree and again
   against the unpacked archive: tests/c/smoke.c built as C11 and C++17 with
   `pkg-config --cflags --libs openbim_ifc` and with `openbim_ifc-static`,
   and run. With the tree moved away, the shared builds must fail to start
   and the static ones must still run. Windows/MSVC is CMake-only and skips
   this step.

Usage:
    check-cmake.py [--build-type Release] [--archive-dir DIR]
"""

from __future__ import annotations

import argparse
import os
import platform
import re
import shlex
import shutil
import subprocess
import sys
import tarfile
import tempfile
import zipfile
from pathlib import Path

CRATE = Path(__file__).resolve().parents[1]
ROOT = CRATE.parents[1]
CONSUMER = CRATE / "tests" / "cmake-consumer"
SMOKE = CRATE / "tests" / "c" / "smoke.c"
WINDOWS = platform.system() == "Windows"
MACOS = platform.system() == "Darwin"

if WINDOWS:
    SHARED_FILES = ["bin/openbim_ifc_capi.dll", "lib/openbim_ifc_capi.dll.lib"]
    STATIC_FILES = ["lib/openbim_ifc_capi.lib"]
elif MACOS:
    SHARED_FILES = ["lib/libopenbim_ifc_capi.dylib"]
    STATIC_FILES = ["lib/libopenbim_ifc_capi.a"]
else:
    SHARED_FILES = ["lib/libopenbim_ifc_capi.so"]
    STATIC_FILES = ["lib/libopenbim_ifc_capi.a"]
PACKAGE_FILES = [
    "include/openbim_ifc.h",
    "lib/cmake/openbim_ifc/openbim_ifc-config.cmake",
    "lib/cmake/openbim_ifc/openbim_ifc-config-version.cmake",
    "share/doc/openbim_ifc/LICENSE",
    "share/doc/openbim_ifc/NOTICE",
    *SHARED_FILES,
    *STATIC_FILES,
]
PKGCONFIG_MODULES = {"shared": "openbim_ifc", "static": "openbim_ifc-static"}
if not WINDOWS:
    PACKAGE_FILES += [f"lib/pkgconfig/{m}.pc" for m in PKGCONFIG_MODULES.values()]


def run(*args: object, cwd: Path | None = None, check: bool = True) -> int:
    command = [str(a) for a in args]
    print("+ " + " ".join(command), flush=True)
    return subprocess.run(command, cwd=cwd, check=check).returncode


def crate_version() -> str:
    text = (CRATE / "Cargo.toml").read_text(encoding="utf-8")
    found = re.search(r'^version = "([^"]+)"', text, re.MULTILINE)
    if not found:
        raise SystemExit("no version in crates/openbim-ifc-capi/Cargo.toml")
    return found.group(1)


def host_triple() -> str:
    out = subprocess.run(["rustc", "-vV"], capture_output=True, text=True, check=True).stdout
    return next(line.split()[1] for line in out.splitlines() if line.startswith("host:"))


def cargo_target_dir() -> Path:
    env = os.environ.get("CARGO_TARGET_DIR")
    return Path(env) if env else ROOT / "target"


def configure_build(source: Path, build: Path, build_type: str, *defines: str) -> None:
    run("cmake", "-S", source, "-B", build, f"-DCMAKE_BUILD_TYPE={build_type}", *defines)
    run("cmake", "--build", build, "--config", build_type, "--parallel")


def ctest(build: Path, build_type: str) -> None:
    run("ctest", "--test-dir", build, "-C", build_type, "--output-on-failure")


def executable(build: Path, build_type: str, name: str) -> Path:
    suffix = ".exe" if WINDOWS else ""
    for candidate in (build / build_type / f"{name}{suffix}", build / f"{name}{suffix}"):
        if candidate.exists():
            return candidate
    raise SystemExit(f"{name} was not built in {build}")


def source_tree(work: Path, build_type: str) -> None:
    """add_subdirectory, configured before cargo has produced the library."""
    profile = cargo_target_dir() / ("debug" if build_type == "Debug" else "release")
    for name in ("libopenbim_ifc_capi.so", "libopenbim_ifc_capi.dylib",
                 "openbim_ifc_capi.dll", "openbim_ifc_capi.dll.lib"):
        # Cargo relinks a unit whose output is missing, so this costs one link.
        (profile / name).unlink(missing_ok=True)
    build = work / "consumer-source"
    configure_build(CONSUMER, build, build_type,
                    f"-DOPENBIM_IFC_SOURCE_TREE={CRATE}", "-DOPENBIM_IFC_LINKAGE=SHARED",
                    f"-DOPENBIM_IFC_CARGO_TARGET_DIR={cargo_target_dir()}")
    ctest(build, build_type)


def anonymous(info: tarfile.TarInfo) -> tarfile.TarInfo:
    """No build-machine user names in a release archive."""
    info.uid = info.gid = 0
    info.uname = info.gname = ""
    return info


def package(work: Path, build_type: str, archive_dir: Path | None) -> Path:
    """Install the package, pack it, and return where the archive unpacked."""
    prefix = work / "install"
    build = work / "package-build"
    # The workspace's own target directory: the libraries are built once.
    configure_build(CRATE, build, build_type,
                    f"-DOPENBIM_IFC_CARGO_TARGET_DIR={cargo_target_dir()}")
    run("cmake", "--install", build, "--config", build_type, "--prefix", prefix)
    missing = [f for f in PACKAGE_FILES if not (prefix / f).is_file()]
    if missing:
        raise SystemExit(f"install tree lacks {missing}")
    pkgconfig_consumers(work / "pkgconfig-installed", prefix)

    stem = f"openbim-ifc-capi-v{crate_version()}-{host_triple()}"
    out = archive_dir or work / "dist"
    out.mkdir(parents=True, exist_ok=True)
    if WINDOWS:
        archive = out / f"{stem}.zip"
        with zipfile.ZipFile(archive, "w", zipfile.ZIP_DEFLATED) as zf:
            for path in sorted(prefix.rglob("*")):
                if path.is_file():
                    zf.write(path, f"{stem}/{path.relative_to(prefix).as_posix()}")
    else:
        archive = out / f"{stem}.tar.gz"
        with tarfile.open(archive, "w:gz") as tf:
            for path in sorted(prefix.rglob("*")):
                if path.is_file():
                    tf.add(path, f"{stem}/{path.relative_to(prefix).as_posix()}",
                           recursive=False, filter=anonymous)
    print(f"packed {archive}")

    unpacked = work / "unpacked"
    if WINDOWS:
        with zipfile.ZipFile(archive) as zf:
            zf.extractall(unpacked)
    else:
        with tarfile.open(archive) as tf:
            tf.extractall(unpacked, filter="data")
    for pc in (unpacked / stem / "lib" / "pkgconfig").glob("*.pc"):
        if str(prefix) in pc.read_text(encoding="utf-8"):
            raise SystemExit(f"{pc.name} names the install prefix {prefix}")
    # The install prefix is gone before any consumer runs: nothing may
    # point back into it.
    shutil.rmtree(prefix)
    return unpacked / stem


def shared_consumer(work: Path, build_type: str, prefix: Path) -> None:
    build = work / "consumer-shared"
    configure_build(CONSUMER, build, build_type,
                    f"-DCMAKE_PREFIX_PATH={prefix}", "-DOPENBIM_IFC_LINKAGE=SHARED")
    ctest(build, build_type)
    if MACOS:
        # dyld honours DYLD_LIBRARY_PATH even for an absolute install name,
        # so the relocation run below cannot see one; read it directly.
        out = subprocess.run(["otool", "-D", str(prefix / SHARED_FILES[0])],
                             capture_output=True, text=True, check=True).stdout
        if "@rpath/libopenbim_ifc_capi.dylib" not in out.splitlines()[1:]:
            raise SystemExit(f"install name is not @rpath/libopenbim_ifc_capi.dylib:\n{out}")

    # Move the library away. A consumer that links it dynamically can no
    # longer start, and must start again once the platform's library search
    # path names the new place: that only works when the library records a
    # bare name (SONAME / @rpath) instead of the path it was built at.
    smoke = executable(build, build_type, "smoke_c")
    moved = prefix.with_name(prefix.name + "-moved")
    if WINDOWS:
        (smoke.parent / "openbim_ifc_capi.dll").unlink()
        search = ("PATH", moved / "bin")
    else:
        search = ("DYLD_LIBRARY_PATH" if MACOS else "LD_LIBRARY_PATH", moved / "lib")
    prefix.rename(moved)
    try:
        if run(smoke, check=False) == 0:
            raise SystemExit("smoke_c ran without its shared library: not dynamically linked")
        env = dict(os.environ)
        env[search[0]] = os.pathsep.join(filter(None, [str(search[1]), env.get(search[0])]))
        print(f"+ {search[0]}={search[1]} {smoke}", flush=True)
        if subprocess.run([str(smoke)], env=env).returncode != 0:
            raise SystemExit("smoke_c does not find its moved shared library through "
                             f"{search[0]}: the library is not relocatable")
    finally:
        moved.rename(prefix)
    print("shared consumer loads the shared library by name: ok")


def static_consumer(work: Path, build_type: str, prefix: Path) -> None:
    # A copy without the shared library: linking it by mistake fails.
    static_prefix = work / "static-only"
    shutil.copytree(prefix, static_prefix)
    for name in SHARED_FILES:
        (static_prefix / name).unlink()
    build = work / "consumer-static"
    configure_build(CONSUMER, build, build_type,
                    f"-DCMAKE_PREFIX_PATH={static_prefix}", "-DOPENBIM_IFC_LINKAGE=STATIC")
    ctest(build, build_type)


def pkg_config(prefix: Path, *args: str) -> str:
    """pkg-config that sees only this prefix's .pc files."""
    env = dict(os.environ)
    env["PKG_CONFIG_LIBDIR"] = env["PKG_CONFIG_PATH"] = str(prefix / "lib" / "pkgconfig")
    env.pop("PKG_CONFIG_SYSROOT_DIR", None)
    print("+ pkg-config " + " ".join(args), flush=True)
    return subprocess.run(["pkg-config", *args], env=env, capture_output=True,
                          text=True, check=True).stdout.strip()


def pkgconfig_consumers(build: Path, prefix: Path) -> None:
    """Build and run the smoke test with nothing but pkg-config's flags (#325)."""
    if WINDOWS:
        print("pkg-config: skipped on Windows; the MSVC package is CMake-only")
        return
    if not shutil.which("pkg-config"):
        raise SystemExit("pkg-config not found: install pkg-config or pkgconf "
                         "(the pkg-config files are checked on Linux and macOS)")
    build.mkdir(parents=True)
    for module in PKGCONFIG_MODULES.values():
        version = pkg_config(prefix, "--modversion", module)
        if version != crate_version():
            raise SystemExit(f"{module}.pc has version {version}, the crate {crate_version()}")
        # ${pcfiledir}: the prefix is wherever the tree is now.
        found = Path(pkg_config(prefix, "--variable=prefix", module)).resolve()
        if found != prefix.resolve():
            raise SystemExit(f"{module}.pc resolves its prefix to {found}, not {prefix}")

    built: dict[str, list[Path]] = {"shared": [], "static": []}
    for variant, module in PKGCONFIG_MODULES.items():
        flags = shlex.split(pkg_config(prefix, "--cflags", module))
        libs = shlex.split(pkg_config(prefix, "--libs", module))
        if variant == "shared":
            # pkg-config sets no runtime path, as for any library; the
            # consumer names one (on macOS the install name is @rpath/...).
            libs.append("-Wl,-rpath," + pkg_config(prefix, "--variable=libdir", module))
        if MACOS:
            libs.append("-Wl,-undefined,error")
        warn = ["-Wall", "-Wextra", "-Werror"]
        c_out = build / f"smoke_c_{variant}"
        run(os.environ.get("CC", "cc"), "-std=c11", *warn, *flags, SMOKE, "-o", c_out, *libs)
        cxx_out = build / f"smoke_cxx_{variant}"
        run(os.environ.get("CXX", "c++"), "-std=c++17", *warn, *flags,
            "-x", "c++", SMOKE, "-x", "none", "-o", cxx_out, *libs)
        for smoke in (c_out, cxx_out):
            run(smoke)
            built[variant].append(smoke)

    # With the tree gone, a shared build cannot start and a static one,
    # which carries the library, still runs: each linked what it claims to.
    moved = prefix.with_name(prefix.name + "-moved")
    prefix.rename(moved)
    try:
        for smoke in built["shared"]:
            if run(smoke, check=False) == 0:
                raise SystemExit(f"{smoke.name} ran without its shared library: "
                                 "openbim_ifc.pc does not link it dynamically")
        for smoke in built["static"]:
            if run(smoke, check=False) != 0:
                raise SystemExit(f"{smoke.name} needs the installed tree: "
                                 "openbim_ifc-static.pc does not link statically")
    finally:
        moved.rename(prefix)
    print(f"pkg-config consumers against {prefix}, shared and static: ok")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--build-type", default="Release", choices=["Release", "Debug"])
    parser.add_argument("--archive-dir", type=Path,
                        help="keep the release archive here (default: discarded)")
    args = parser.parse_args()
    archive_dir = args.archive_dir.resolve() if args.archive_dir else None

    with tempfile.TemporaryDirectory(prefix="openbim-ifc-cmake-") as tmp:
        work = Path(tmp)
        source_tree(work, args.build_type)
        prefix = package(work, args.build_type, archive_dir)
        pkgconfig_consumers(work / "pkgconfig-unpacked", prefix)
        shared_consumer(work, args.build_type, prefix)
        static_consumer(work, args.build_type, prefix)
    print("openbim_ifc CMake package and pkg-config files: ok")
    return 0


if __name__ == "__main__":
    sys.exit(main())
