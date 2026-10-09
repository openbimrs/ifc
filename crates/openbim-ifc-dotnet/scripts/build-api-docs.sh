#!/usr/bin/env bash
# The hosted .NET API reference (#385): docfx renders the OpenBim.Ifc
# assembly's XML documentation comments into a static site.
#
#   build-api-docs.sh            # build and check it (gate)
#   build-api-docs.sh OUT_DIR    # ... and copy the site to OUT_DIR (Pages)
#
# docfx is pinned as a dotnet local tool in .config/dotnet-tools.json. The
# package builds with GenerateDocumentationFile and TreatWarningsAsErrors, so
# a public member without a doc comment (CS1591) already fails the build;
# docfx then runs with --warningsAsErrors, and every API page it extracted
# must carry a summary.
#
# Without a .NET SDK the reference is skipped with a message, as the gate's
# other optional reference builds are, unless IFC_DOTNET_REQUIRED is set
# (the Pages workflow sets it).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
CRATE="$ROOT/crates/openbim-ifc-dotnet"
OUT="${1:-}"

if ! command -v dotnet >/dev/null 2>&1; then
    if [[ -n "${IFC_DOTNET_REQUIRED:-}" ]]; then
        echo "error: IFC_DOTNET_REQUIRED is set but no dotnet SDK is on PATH" >&2
        exit 1
    fi
    echo ".NET API reference skipped (no dotnet SDK)"
    exit 0
fi

cd "$ROOT"
export DOTNET_CLI_TELEMETRY_OPTOUT=1 DOTNET_NOLOGO=1
# docfx writes the extracted metadata next to its config (docfx/api/, ignored)
# and the site under the package's ignored artifacts/.
API="$CRATE/docfx/api"
SITE="$CRATE/dotnet/artifacts/docfx/site"
rm -rf "$API" "$SITE"

dotnet tool restore
# The net8.0 assembly docfx reads; restores the project for it too.
dotnet build "$CRATE/dotnet/OpenBim.Ifc/OpenBim.Ifc.csproj" -c Release -f net8.0 --nologo
dotnet tool run docfx "$CRATE/docfx/docfx.json" --warningsAsErrors

# Every extracted type and member must have a summary: the compiler checks
# declared members, this checks what the reference actually shows.
python3 - "$API" <<'PY'
import pathlib
import sys

api = pathlib.Path(sys.argv[1])
pages = sorted(api.glob("OpenBim.Ifc*.yml"))
if not pages:
    sys.exit(f"docfx extracted no OpenBim.Ifc pages into {api}")
missing = []
for page in pages:
    lines = page.read_text(encoding="utf-8").splitlines()
    try:
        end = lines.index("references:")
    except ValueError:
        end = len(lines)
    item, has_summary, kind = None, False, None
    def close():
        if item is not None and kind != "Namespace" and not has_summary:
            missing.append(f"{page.name}: {item}")
    for line in lines[:end]:
        if line.startswith("- uid: "):
            close()
            item, has_summary, kind = line[len("- uid: "):], False, None
        elif line.startswith("  summary:"):
            has_summary = line[len("  summary:"):].strip() not in ("", "''", '""')
        elif line.startswith("  type: "):
            kind = line[len("  type: "):].strip()
    close()
if missing:
    print("API members without a summary in the .NET reference:", file=sys.stderr)
    print("\n".join(f"  {m}" for m in missing), file=sys.stderr)
    sys.exit(1)
print(f".NET API reference: {len(pages)} pages, every member summarised")
PY

test -f "$SITE/index.html"
test -f "$SITE/api/OpenBim.Ifc.IfcModel.html"

if [[ -n "$OUT" ]]; then
    rm -rf "$OUT"
    mkdir -p "$OUT"
    cp -r "$SITE/." "$OUT/"
    echo ".NET API reference copied to $OUT"
fi
