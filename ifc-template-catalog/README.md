# `ifc-template-catalog`

Typed import, validation, query, correction overlays, and deterministic embedded snapshots for buildingSMART PSD/QTO template catalogs.

## IFC4 applicability index

`data/ifc4-add2-tc1.tsv` is a diffable machine-readable index generated from the official embedded IFC4 ADD2 TC1 snapshot. It maps applicable entity and predefined type to each property or quantity set member, including nested property paths, value forms, source XML paths, and digests. For quantities, `member_kind` is the normalized query category and `value_type` preserves the published `Q_LENGTH`, `Q_AREA`, and related XML token; an empty `unit_type` means the source XML declares no explicit unit type.

Regenerate atomically:

```bash
cargo run -p ifc-template-catalog --example export_ifc4_tsv -- \
  ifc-template-catalog/data/ifc4-add2-tc1.tsv
```

`cargo test -p ifc-template-catalog --all-features` byte-compares fresh output with the committed artifact. Applicability rows retain the published selector and declare subtype applicability; consumers needing an exact concrete entity query should use the schema-aware catalog query API.
