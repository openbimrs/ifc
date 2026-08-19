#!/usr/bin/env python3
"""Measure OCCT module/toolkit sizes (source LOC + Debian binary .so sizes)."""
import os, subprocess, json

OCCT = "/home/friedrich/occt-research/occt"
DEB = "/tmp/occtdeb/ex/usr/lib/x86_64-linux-gnu"
EXTS = (".cxx", ".hxx", ".h", ".c", ".lxx", ".pxx", ".cpp")

def walk(root, skip_gtests=True):
    files = 0
    lines = 0
    for dp, dn, fn in os.walk(root):
        if skip_gtests and os.sep + "GTests" in dp:
            continue
        for f in fn:
            if f.endswith(EXTS):
                p = os.path.join(dp, f)
                files += 1
                try:
                    with open(p, "rb") as fh:
                        lines += fh.read().count(b"\n")
                except OSError:
                    pass
    return files, lines

# --- per module ---
print("=" * 78)
print("OCCT SOURCE SIZE BY MODULE (GTests excluded)")
print("=" * 78)
print(f"{'Module':<26}{'pkgs':>6}{'toolkits':>10}{'files':>8}{'lines':>10}")
src = os.path.join(OCCT, "src")
mod_tot = [0, 0]
for m in sorted(os.listdir(src)):
    md = os.path.join(src, m)
    if not os.path.isdir(md):
        continue
    tks = [d for d in os.listdir(md) if d.startswith("TK") and os.path.isdir(os.path.join(md, d))]
    pkgs = 0
    for tk in tks:
        pkgs += len([d for d in os.listdir(os.path.join(md, tk))
                     if os.path.isdir(os.path.join(md, tk, d)) and d != "GTests"])
    f, l = walk(md)
    mod_tot[0] += f
    mod_tot[1] += l
    print(f"{m:<26}{pkgs:>6}{len(tks):>10}{f:>8}{l:>10}")
print(f"{'TOTAL':<26}{'':>6}{'':>10}{mod_tot[0]:>8}{mod_tot[1]:>10}")

# --- per toolkit ---
tk_src = {}
for m in os.listdir(src):
    md = os.path.join(src, m)
    if not os.path.isdir(md):
        continue
    for tk in os.listdir(md):
        p = os.path.join(md, tk)
        if tk.startswith("TK") and os.path.isdir(p):
            tk_src[tk] = walk(p)

# --- binary sizes ---
so = {}
if os.path.isdir(DEB):
    for f in os.listdir(DEB):
        if f.startswith("libTK") and ".so." in f and f.count(".") >= 3:
            name = f.split(".so")[0][3:]
            so[name] = os.path.getsize(os.path.join(DEB, f))

SETS = {
  "ALL shipped .so (Debian 7.8.1 runtime pkgs)": sorted(so),
  "IfcOpenShell FindOpenCASCADE.cmake list": """TKernel TKMath TKBRep TKGeomBase TKGeomAlgo TKG3d TKG2d
     TKShHealing TKTopAlgo TKMesh TKPrim TKBool TKBO TKFillet TKXSBase TKOffset TKHLR TKBin
     TKDESTEP TKDEIGES""".split(),
  "  ...minus STEP/IGES/XSBase/Bin (no CAD file IO)": """TKernel TKMath TKBRep TKGeomBase TKGeomAlgo
     TKG3d TKG2d TKShHealing TKTopAlgo TKMesh TKPrim TKBool TKBO TKFillet TKOffset TKHLR""".split(),
  "Minimal IFC->triangles (link-closed, w/ booleans)": """TKernel TKMath TKG2d TKG3d TKGeomBase
     TKGeomAlgo TKBRep TKTopAlgo TKPrim TKShHealing TKMesh TKBO TKBool""".split(),
  "Floor: build+eval+tessellate, NO booleans": """TKernel TKMath TKG2d TKG3d TKGeomBase TKGeomAlgo
     TKBRep TKTopAlgo TKPrim TKMesh""".split(),
}

print()
print("=" * 78)
print("BINARY SIZE (Debian trixie libocct-* 7.8.1+dfsg1-3, stripped shared libs, amd64)")
print("=" * 78)
allsz = sum(so.values())
for label, names in SETS.items():
    s = sum(so.get(n, 0) for n in names)
    missing = [n for n in names if n not in so]
    srcf = sum(tk_src.get(n, (0, 0))[0] for n in names)
    srcl = sum(tk_src.get(n, (0, 0))[1] for n in names)
    print(f"{s/1048576:7.1f} MB  ({100*s/allsz:5.1f}%)  {len(names):>3} libs  "
          f"| src {srcf:>5} files {srcl:>8} lines  | {label}")
    if missing:
        print(f"{'':>12}(no .so found for: {' '.join(missing)})")

print()
print("Largest single shipped libs:")
for n, s in sorted(so.items(), key=lambda x: -x[1])[:12]:
    print(f"  {s/1048576:6.1f} MB  {n}")
