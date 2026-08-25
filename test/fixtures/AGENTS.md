# AGENTS.md — test/fixtures/

Read [`../../AGENTS.md`](../../AGENTS.md) first. Active fixture work belongs only
in sibling `PLAN.md`; load it only when changing this directory.

Small `.ifc` files for use in crate tests, copied from upstream reference repos.
Total ~380 KiB, 20 files —
kept intentionally small; this is a curated edge-case set, not a bulk corpus.

## Layout

- `ifclite-geometry/` — 11 files from `ifc-lite`'s
  `rust/geometry/tests/fixtures/` and `rust/processing/tests/fixtures/`
  (MPL-2.0, github.com/LTplus-AG/ifc-lite). Geometry edge cases: mapped-item
  cycles/nesting, swept-disk composite-curve profiles, shared-point faceted
  breps, CSG, halfspace flyaway, scaled units, overlapping wall openings.
  Filenames match upstream (`issue_NNNN_*`, `mapped_instances_*`,
  `swept_disk_*`) — keep that convention when adding more so provenance stays
  traceable by name alone.
- `ifcopenshell-validate/` — 8 files from `ifcopenshell`'s
  `src/ifcopenshell-python/test/fixtures/validate/` (LGPL-3.0-or-later,
  github.com/IfcOpenShell/IfcOpenShell). Schema/header validation edge cases:
  `pass-*` / `fail-*` pairs for duplicated GUIDs, selected simple types,
  complex numbers, malformed headers. These are IFC-spec-conformance cases,
  not geometry.

## Agent rules

1. **These are third-party test data.** Preserve upstream provenance and license
   terms; do not add a fixture that looks like a real client export rather than
   a synthetic/minimal repro case.
2. **Keep the `pass-`/`fail-`/`issue_NNNN_` naming from upstream** when you add
   more files from the same source — it's how you trace a fixture back to the
   upstream test it validates without a separate manifest.
3. **Add new fixtures deliberately, not in bulk.** The point of this directory
   is edge-case coverage per IFC construct, not corpus size. Before adding a
   new file, check it actually exercises something the existing 20 don't.
4. When a crate test loads one of these, reference it by relative path from
   the crate (`../test/fixtures/<subdir>/<file>.ifc`) and name the test
   after the fixture so a failing test is self-describing.
5. Full oracle-scale `.ifc` corpora (e.g. Solibri's example models) belong to
   external local oracle storage, not here — this repo's fixtures stay
   small and hand-picked.
