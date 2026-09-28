# ifc-template-catalog

Versioned buildingSMART PSD/QTO template catalogs: typed property and
quantity set definitions with their applicability, provenance and
diagnostics, deterministic embedded snapshots, and explicit correction
overlays. It owns no IFC model records (ADR 0017).

```bash
cargo add ifc-template-catalog
```

The [`openbim-ifc`](https://crates.io/crates/openbim-ifc) facade also provides
it behind its `property-catalog` feature.

- API documentation: [docs.rs/ifc-template-catalog](https://docs.rs/ifc-template-catalog)
- Reference page: [openbimrs.github.io/ifc](https://openbimrs.github.io/ifc/reference/crates/ifc-template-catalog)
- Source and issues: [github.com/openbimrs/ifc](https://github.com/openbimrs/ifc)

## Embedded editions and profiles

Official snapshots are embedded for IFC2X3 TC1, IFC4 ADD2 TC1, and IFC4X3
ADD2. Corrected overlays exist for IFC4 ADD2 TC1 and IFC4X3 ADD2; IFC2X3
TC1 has the official profile only. Official artifacts are never rewritten:
the IFC4X3 overlay adds `Pset_Stationing.HasIncreasingStation`, which the
published ADD2 documentation lists and the PSD XML the snapshot is built
from omits.

## Version-explicit TSV index

Each committed TSV has an `edition` and `source_digest` column, preserves
set/member GUIDs when the source publishes them, and retains the source XML
path and digest. GUIDs are release-scoped evidence, not a promise that equal
names or GUIDs have identical semantics across editions. Generate one
deterministically:

```bash
cargo run -p ifc-template-catalog --example export_ifc4_tsv -- \
  ifc4x3-add2 crates/ifc-template-catalog/data/ifc4x3-add2.tsv
```

The example accepts `ifc2x3-tc1`, `ifc4-add2-tc1`, or `ifc4x3-add2`.
Provenance, exact corpus gates, checksums, and artifact sizes are in
`data/NOTICE.md`.
