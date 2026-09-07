#!/usr/bin/env bash
# Fetch the three normative EXPRESS schemas the schema-backed tests need.
#
# references/ifc-spec/ is a symlink to bulk storage and is NOT committed:
# the schemas are CC BY-ND 4.0, so this repo does not redistribute them.
# Without them every schema-backed test silently skips, which is how an
# IFC2X3 slot-name bug reached main with CI green.
#
# ND permits verbatim redistribution but forbids derivatives, so CI
# fetches upstream at run time rather than vendoring a subset.
#
# Integrity is pinned on LINE-ENDING-NORMALISED content: upstream has
# served both CRLF and LF for the same schema, and a cosmetic change
# must not break CI. Content changes still fail loudly.
set -euo pipefail

DEST="${1:-references/ifc-spec}"
UA="Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 Chrome/120 Safari/537.36"
BASE="${IFC_SPEC_BASE:-https://standards.buildingsmart.org/IFC/RELEASE}"
# The archive indexes the canonical origin, independent of BASE.
ORIGIN="https://standards.buildingsmart.org/IFC/RELEASE"
ARCHIVE="https://web.archive.org/web"

# rel-path | sha256 of LF-normalised bytes | url
SCHEMAS="
ifc2x3-tc1/IFC2X3_TC1.exp bf89eda341bccec041df37dffb79f0b8d4e3d5411d07541560d05bf69d53fc3a IFC2x3/TC1/EXPRESS/IFC2X3_TC1.exp
ifc4-add2-tc1/IFC4.exp a3e46d39a85c2b683e7167572165d74b4ff6f8ef7e7c1e7f314a4980a63c9edb IFC4/ADD2_TC1/EXPRESS/IFC4.exp
ifc4x3-add2/IFC4X3_ADD2.exp f67c8762b13a099c28082061e6f16b9ef1284ceec34069792afc702725675860 IFC4_3/HTML/IFC4X3_ADD2.exp
"

echo "$SCHEMAS" | while read -r rel want url; do
  [ -n "$rel" ] || continue
  out="$DEST/$rel"
  mkdir -p "$(dirname "$out")"
  if [ -f "$out" ]; then
    have=$(tr -d "\\r" < "$out" | sha256sum | cut -d" " -f1)
    if [ "$have" = "$want" ]; then
      echo "  cached  $rel"
      continue
    fi
  fi
  # Two sources, both verified against the SAME pinned checksum, so a
  # mirror can never substitute different content:
  #   1. buildingSMART, the normative origin. It 403s without a browser
  #      User-Agent AND 403s from datacenter IPs, so GitHub-hosted
  #      runners cannot reach it at all.
  #   2. the Wayback Machine at a pinned year, whose id_ form returns the
  #      original unmodified bytes and is reachable from CI.
  got=""
  for src in "$BASE/$url" "$ARCHIVE/2023id_/$ORIGIN/$url"; do
    # Quiet on failure: an unreachable primary is expected on CI runners,
    # and only the final all-sources-failed case is worth reporting.
    curl -fsSL -A "$UA" --retry 2 --retry-delay 2 --max-time 120 \
      -o "$out.tmp" "$src" 2>/dev/null || continue
    got=$(tr -d "\\r" < "$out.tmp" | sha256sum | cut -d" " -f1)
    [ "$got" = "$want" ] && break
    got=""
  done
  if [ "$got" != "$want" ]; then
    rm -f "$out.tmp"
    echo "FATAL: could not obtain $rel with checksum $want" >&2
    exit 1
  fi
  mv "$out.tmp" "$out"
  echo "  fetched $rel"
done
