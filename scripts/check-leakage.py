#!/usr/bin/env python3
"""Fail if public source or build artifacts leak standards payloads."""

from __future__ import annotations

import os
import subprocess
import sys
import tarfile
import zipfile
from pathlib import Path, PurePosixPath

ROOT = Path(os.environ.get("REPO_ROOT", Path(__file__).resolve().parents[1])).resolve()
FORBIDDEN_SUFFIXES = {".xsd", ".pdf"}
FORBIDDEN_PATH_PARTS = {"references", "schemas"}
XSD_MARKERS = (
    b"<xs:" + b"schema",
    b"<xsd:" + b"schema",
    b"<schema " + b'xmlns="http://www.w3.org/2001/XMLSchema"',
)
PDF_MARKERS = (b"%PDF-",)
DERIVED_ROOTS = {".git", ".venv", "node_modules", "target"}
DERIVED_PATHS = {("docs", ".vitepress", "cache"), ("docs", ".vitepress", "dist")}


def fail(message: str) -> None:
    print(f"leakage: {message}", file=sys.stderr)
    raise SystemExit(1)


def check_name(name: str, origin: str) -> None:
    path = PurePosixPath(name.replace("\\", "/"))
    if path.suffix.lower() in FORBIDDEN_SUFFIXES:
        fail(f"forbidden standards file in {origin}: {name}")
    if any(part.lower() in FORBIDDEN_PATH_PARTS for part in path.parts):
        fail(f"forbidden standards/reference directory in {origin}: {name}")


def check_bytes(name: str, payload: bytes, origin: str) -> None:
    if any(marker.lower() in payload.lower() for marker in XSD_MARKERS):
        fail(f"XSD schema bytes found in {origin}: {name}")
    if any(marker in payload.lstrip()[:16] for marker in PDF_MARKERS):
        fail(f"PDF bytes found in {origin}: {name}")


def check_directory(path: Path) -> None:
    for item in path.rglob("*"):
        if item.is_file():
            relative = item.relative_to(path).as_posix()
            check_name(relative, str(path))
            check_bytes(relative, item.read_bytes(), str(path))


def check_archive(path: Path) -> None:
    if zipfile.is_zipfile(path):
        with zipfile.ZipFile(path) as archive:
            for info in archive.infolist():
                if info.is_dir():
                    continue
                check_name(info.filename, str(path))
                check_bytes(info.filename, archive.read(info), str(path))
        return
    try:
        with tarfile.open(path, "r:*") as archive:
            for member in archive.getmembers():
                if not member.isfile():
                    continue
                check_name(member.name, str(path))
                handle = archive.extractfile(member)
                assert handle is not None
                check_bytes(member.name, handle.read(), str(path))
    except tarfile.TarError as error:
        fail(f"unsupported artifact {path}: {error}")


def source_candidates() -> list[str]:
    result = subprocess.run(
        ["git", "ls-files", "--cached", "--others", "--exclude-standard"],
        cwd=ROOT,
        check=False,
        capture_output=True,
        text=True,
    )
    if result.returncode == 0:
        return [line for line in result.stdout.splitlines() if line]
    if (ROOT / ".git").exists():
        fail(f"could not enumerate Git source files: {result.stderr.strip()}")
    candidates = []
    for item in ROOT.rglob("*"):
        relative = item.relative_to(ROOT)
        if not item.is_file() or relative.parts[0] in DERIVED_ROOTS:
            continue
        if any(relative.parts[: len(path)] == path for path in DERIVED_PATHS):
            continue
        candidates.append(relative.as_posix())
    return sorted(candidates)


if len(sys.argv) == 1:
    for name in source_candidates():
        check_name(name, "public source tree")
        candidate = ROOT / name
        if candidate.is_file():
            if zipfile.is_zipfile(candidate) or tarfile.is_tarfile(candidate):
                check_archive(candidate)
            else:
                check_bytes(name, candidate.read_bytes(), "public source tree")
    print("leakage: PASS (source names and payloads)")
else:
    for argument in sys.argv[1:]:
        candidate = Path(os.path.abspath(argument))
        if candidate.is_dir():
            check_directory(candidate)
        elif candidate.is_file():
            check_archive(candidate)
        else:
            fail(f"artifact does not exist: {candidate}")
    print(f"leakage: PASS ({len(sys.argv) - 1} artifact(s))")
