#!/usr/bin/env bash
# Build the openbim-ifc wheel with maturin, install it into a throwaway uv
# venv, and run the Python suites against it. Used by the gate and CI.
#
# Needs `uv` and `maturin` on PATH (CI pins both; see .github/workflows).
set -euo pipefail

crate_dir="$(cd "$(dirname "$0")/.." && pwd)"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

for tool in uv maturin; do
    command -v "$tool" >/dev/null || { echo "error: $tool not on PATH" >&2; exit 1; }
done

uv venv --quiet --python "${PYTHON:-python3}" "$work/venv"

# A real release wheel, installed the way a user would get it, with the
# `pandas` extra, plus mypy and the pandas stubs for the typing check
# (#332). Pinned so a new release cannot change the verdict unseen.
maturin build --quiet --release --manifest-path "$crate_dir/Cargo.toml" --out "$work/dist"
wheel="$(ls "$work"/dist/*.whl)"
uv pip install --quiet --python "$work/venv/bin/python" "$wheel[pandas]" \
    "pandas==3.0.6" "pandas-stubs==3.0.5.260914" "mypy==2.4.0"
ls "$work"/dist

# The extras are installed, so the suites that would skip without them
# (the pandas export, mypy --strict) must run.
export OPENBIM_IFC_REQUIRE_EXTRAS=1

# Run from outside the source tree so the tests import the installed wheel,
# not the python/ sources beside them. No pipe: the exit status is the verdict.
cd "$work"
"$work/venv/bin/python" -m unittest discover \
    -s "$crate_dir/tests/python" -t "$crate_dir/tests/python" -v

# The docs site's Python reference (#331): pdoc must render the installed
# package, as the Pages workflow does. Pinned with the workflow's version.
uv pip install --quiet --python "$work/venv/bin/python" "pdoc==16.0.0"
"$work/venv/bin/python" -m pdoc --no-show-source -o "$work/pdoc" openbim_ifc
test -f "$work/pdoc/openbim_ifc.html"

# The opt-in mesh wheel (#328): the published wheel above refuses meshes, so
# build one with `--features mesh`, replace the installed wheel with it and
# run the geometry suite again, now with its mesh tests.
maturin build --quiet --release --manifest-path "$crate_dir/Cargo.toml" \
    --features mesh,graph --out "$work/dist-mesh"
uv pip install --quiet --python "$work/venv/bin/python" --reinstall "$(ls "$work"/dist-mesh/*.whl)"
OPENBIM_IFC_MESH=1 "$work/venv/bin/python" -m unittest discover \
    -s "$crate_dir/tests/python" -t "$crate_dir/tests/python" -p "test_geometry.py" -v
# The cookbook's mesh recipe (#331) needs the same wheel.
OPENBIM_IFC_MESH=1 "$work/venv/bin/python" -m unittest discover \
    -s "$crate_dir/tests/python" -t "$crate_dir/tests/python" -p "test_cookbook.py" -v
