# `ifc-template-catalog`

Typed import, validation, query, correction overlays, and deterministic embedded snapshots for buildingSMART PSD/QTO template catalogs.

## Embedded editions and profiles

Official snapshots are embedded for IFC2X3 TC1, IFC4 ADD2 TC1, and IFC4X3 ADD2. Corrected overlays are deliberately available only for IFC4 ADD2 TC1; official artifacts are never rewritten.

## Version-explicit TSV index

Each committed TSV has an `edition` and `source_digest` column, preserves
set/member GUIDs when the source publishes them, and retains the source XML path
and digest. GUIDs are release-scoped evidence, not a promise that equal names or
GUIDs have identical semantics across editions. Generate one deterministically:

```bash
cargo run -p ifc-template-catalog --example export_ifc4_tsv -- \
  ifc4x3-add2 ifc-template-catalog/data/ifc4x3-add2.tsv
```

The example accepts `ifc2x3-tc1`, `ifc4-add2-tc1`, or `ifc4x3-add2`. Provenance, exact corpus gates, checksums, and artifact sizes are in `data/NOTICE.md`.
