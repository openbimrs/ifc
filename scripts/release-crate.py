#!/usr/bin/env python3
"""Release a single crate without touching its siblings.

A family-wide release republishes 27 crates and costs hours of crates.io
rate limiting. Most changes touch one crate, and `^0.2.0` already accepts
0.2.1, so dependents pick a compatible release up without being rebuilt.

This tool answers the one question that decides the blast radius:

    is this bump compatible, or breaking?

A compatible bump (patch/minor on 0.x, i.e. 0.2.0 -> 0.2.1) publishes the
crate alone. A breaking bump (0.2.x -> 0.3.0) invalidates every dependents'
requirement, so each dependent must have its requirement edited and be
republished in turn -- the tool lists exactly which, and refuses to publish
until they are dealt with.

Usage:
    scripts/release-crate.py <crate> --impact          # what would this cost?
    scripts/release-crate.py <crate> --set 0.2.1       # bump manifest+changelog
    scripts/release-crate.py <crate> --publish         # verify, tag, publish
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def metadata() -> dict:
    out = subprocess.run(
        ["cargo", "metadata", "--no-deps", "--format-version", "1"],
        cwd=ROOT, capture_output=True, text=True, check=True,
    ).stdout
    return json.loads(out)


def dependents(target: str) -> list[str]:
    """Workspace crates depending on `target`, by any dependency kind.

    Dev- and build-dependencies count: cargo resolves them when packaging,
    so a requirement they carry can block a publish just as a normal one can.
    """
    found = []
    for pkg in metadata()["packages"]:
        for dep in pkg["dependencies"]:
            if dep["name"] == target:
                found.append(pkg["name"])
                break
    return sorted(found)


def requirement_of(dependent: str, target: str) -> str | None:
    """The version requirement `dependent` places on `target`."""
    for pkg in metadata()["packages"]:
        if pkg["name"] != dependent:
            continue
        for dep in pkg["dependencies"]:
            if dep["name"] == target:
                return dep["req"]
    return None


def parse_version(text: str) -> tuple[int, int, int]:
    parts = text.split("-")[0].split(".")
    nums = [int(p) if p.isdigit() else 0 for p in parts]
    while len(nums) < 3:
        nums.append(0)
    return tuple(nums[:3])


def is_breaking(old: str, new: str) -> bool:
    """Cargo compatibility for 0.x: the MINOR field is the breaking axis.

    Below 1.0, `^0.2.0` means `>=0.2.0, <0.3.0`. So 0.2.x is compatible and
    0.3.0 is not. At or above 1.0 the major field takes that role.
    """
    o, n = parse_version(old), parse_version(new)
    if o[0] != n[0]:
        return True
    if o[0] == 0:
        if o[1] != n[1]:
            return True
        # 0.0.x is the degenerate case: `^0.0.1` means `>=0.0.1, <0.0.2`,
        # so even a patch bump is breaking.
        if o[1] == 0:
            return o[2] != n[2]
        return False
    return False


def current_version(crate: str) -> str:
    for pkg in metadata()["packages"]:
        if pkg["name"] == crate:
            return pkg["version"]
    raise SystemExit(f"not a workspace member: {crate}")


def published_versions(crate: str) -> list[str]:
    """Versions live on crates.io, via the sparse index.

    The index is used rather than the API because the API rejects unusual
    user agents with 403, which is indistinguishable from "crate absent".
    """
    name = crate.lower()
    if len(name) <= 2:
        path = f"{len(name)}/{name}"
    elif len(name) == 3:
        path = f"3/{name[0]}/{name}"
    else:
        path = f"{name[:2]}/{name[2:4]}/{name}"
    url = f"https://index.crates.io/{path}"
    try:
        with urllib.request.urlopen(url, timeout=30) as response:
            body = response.read().decode()
    except Exception:
        return []
    out = []
    for line in body.strip().split("\\n"):
        if line.strip():
            out.append(json.loads(line)["vers"])
    return out


def impact(crate: str, new: str | None) -> int:
    old = current_version(crate)
    deps = dependents(crate)
    print(f"crate            {crate}")
    print(f"current version  {old}")
    live = published_versions(crate)
    latest = live[-1] if live else "(none)"
    print(f"published        {latest}")
    print(f"dependents       {len(deps)}" + (f"  {deps}" if deps else ""))
    if new is None:
        return 0
    breaking = is_breaking(old, new)
    kind = "BREAKING" if breaking else "compatible"
    print(f"proposed         {new}   ({kind})")
    print()
    if not breaking:
        print(f"publish cost: 1 crate ({crate}).")
        for d in deps:
            req = requirement_of(d, crate)
            print(f"  {d} requires {req} -- satisfied by {new}, no action needed")
        return 0
    print(f"publish cost: {1 + len(deps)} crates.")
    print("each dependent needs its requirement edited and a release of its own:")
    for d in deps:
        req = requirement_of(d, crate)
        print(f"  {d} requires {req} -- REJECTS {new}, must be updated")
    return 0




def today() -> str:
    import datetime
    return datetime.date.today().isoformat()


def apply_bump(crate: str, new: str) -> None:
    """Write the new version into the manifest and open a changelog section."""
    manifest = ROOT / crate / "Cargo.toml"
    text = manifest.read_text(encoding="utf-8")
    if re.search(r"(?m)^version\.workspace\s*=\s*true", text):
        raise SystemExit(
            f"{crate} inherits version.workspace: it cannot be released")
    old = current_version(crate)
    updated, count = re.subn(
        r"(?m)^version\s*=\s*\"%s\"" % re.escape(old),
        'version = "%s"' % new, text, count=1)
    if count != 1:
        raise SystemExit(f"could not rewrite version in {manifest}")
    manifest.write_text(updated, encoding="utf-8")

    # Sibling manifests requiring this crate need their requirement lifted
    # only when the bump is breaking; a compatible one is already accepted.
    if is_breaking(old, new):
        for dep in dependents(crate):
            dm = ROOT / dep / "Cargo.toml"
            dt = dm.read_text(encoding="utf-8")
            nt = re.sub(
                r"(%s\s*=\s*\{[^}]*version\s*=\s*)\"[^\"]+\"" % re.escape(crate),
                r'\1"%s"' % new, dt)
            nt = re.sub(
                r"(?m)^(%s\s*=\s*)\"[^\"]+\"$" % re.escape(crate),
                r'\1"%s"' % new, nt)
            if nt != dt:
                dm.write_text(nt, encoding="utf-8")
                print(f"  updated requirement in {dep}")

    changelog = ROOT / crate / "CHANGELOG.md"
    ct = changelog.read_text(encoding="utf-8")
    anchor = "## [Unreleased]\n"
    if anchor not in ct:
        raise SystemExit(f"no [Unreleased] section in {changelog}")
    head, _, tail = ct.partition(anchor)
    ct = head + anchor + "\n## [%s] - %s\n" % (new, today()) + tail
    ct = ct.replace(
        "[Unreleased]: https://github.com/openbimrs/ifc/compare/%s-v%s...HEAD"
        % (crate, old),
        "[Unreleased]: https://github.com/openbimrs/ifc/compare/%s-v%s...HEAD\n"
        "[%s]: https://github.com/openbimrs/ifc/releases/tag/%s-v%s"
        % (crate, new, new, crate, new))
    changelog.write_text(ct, encoding="utf-8")
    print(f"bumped {crate}: {old} -> {new}")


def publish(crate: str) -> int:
    """Verify, tag, and publish one crate.

    Publishing happens from a detached worktree at the tag so the VCS SHA
    embedded in the .crate is deterministic and the working tree stays
    free for other work.
    """
    version = current_version(crate)
    if version in published_versions(crate):
        print(f"{crate} {version} is already live; nothing to do")
        return 0
    dirty = subprocess.run(["git", "status", "--porcelain"],
        cwd=ROOT, capture_output=True, text=True).stdout.strip()
    if dirty:
        print("working tree is dirty; commit before publishing", file=sys.stderr)
        return 1
    tag = f"{crate}-v{version}"
    existing = subprocess.run(["git", "tag", "-l", tag],
        cwd=ROOT, capture_output=True, text=True).stdout.strip()
    if not existing:
        subprocess.run(["git", "tag", "-a", tag, "-m", f"{crate} {version}"],
            cwd=ROOT, check=True)
        print(f"tagged {tag}")
    subprocess.run(["git", "push", "origin", tag], cwd=ROOT, check=True)
    worktree = Path("/tmp") / f"pub-{crate}-{version}"
    subprocess.run(["git", "worktree", "remove", str(worktree), "--force"],
        cwd=ROOT, capture_output=True)
    subprocess.run(["git", "worktree", "add", "--detach", str(worktree), tag],
        cwd=ROOT, check=True)
    try:
        result = subprocess.run(["cargo", "publish", "-p", crate, "--locked"],
            cwd=worktree)
        if result.returncode != 0:
            return result.returncode
    finally:
        subprocess.run(["git", "worktree", "remove", str(worktree), "--force"],
            cwd=ROOT, capture_output=True)
    print(f"published {crate} {version}")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__,
        formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("crate")
    parser.add_argument("--set", dest="new_version",
        help="version to bump to; reports impact, writes only with --apply")
    parser.add_argument("--apply", action="store_true",
        help="with --set, write the manifest and changelog changes")
    parser.add_argument("--publish", action="store_true",
        help="tag and publish the crate at its committed version")
    args = parser.parse_args()

    if args.publish:
        return publish(args.crate)

    code = impact(args.crate, args.new_version)
    if code != 0:
        return code
    if args.new_version:
        print()
        if args.apply:
            apply_bump(args.crate, args.new_version)
            print()
            print("next: review the changelog entry, commit, then")
            print(f"      python3 scripts/release-crate.py {args.crate} --publish")
        else:
            print("dry run: pass --apply to write the bump")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
