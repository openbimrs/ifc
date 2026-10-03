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
ADD2 (feature `embedded`, default). Corrected overlays exist for IFC4 ADD2
TC1 and IFC4X3 ADD2; IFC2X3 TC1 has the official profile only. Official
artifacts are never rewritten: the IFC4X3 overlay adds
`Pset_Stationing.HasIncreasingStation`, which the published ADD2
documentation lists and the PSD XML the snapshot is built from omits.

## Snapshot format

All three editions live in one container, `data/catalog.bin` (format 3,
module `snapshot`): 1,423,604 bytes, where the three bincode snapshots it
replaced were 3,648,573. It is a lossless format shift, nothing is
trimmed:

- a string table holds each string the distinct records use more than
  once, most used first; a string used once stays inline beside its
  context; a lowercase hexadecimal GUID or SHA-256 is stored as its bytes;
- each distinct property, quantity and set record is stored once across
  editions, so templates and texts IFC4 ADD2 TC1 and IFC4X3 ADD2 publish
  identically take no extra space;
- each edition keeps its own entry: its source manifest (release, source
  URL, source digest, counts) and its sets in order, each with its source
  path and file digest.

The first lookup of an edition decodes that edition's records only. The
container is not compressed on top: it would need a decompressor in every
binary, and every channel the crate ships through compresses already.

| Snapshot | Bytes | gzip -9 | brotli 11 |
| --- | ---: | ---: | ---: |
| IFC2X3 TC1 (`ifc2x3-tc1.bin`) | 325,739 | 105,577 | 87,767 |
| IFC4 ADD2 TC1 (`ifc4-add2-tc1.bin`) | 1,015,315 | 368,172 | 280,574 |
| IFC4X3 ADD2 (`ifc4x3-add2.bin`) | 1,086,526 | 401,730 | 308,355 |
| the container, every edition | 1,423,604 | 547,657 | 394,150 |

`snapshot::encode` writes a container or a per-edition file
deterministically; the SHA-256 of each is pinned (`CONTAINER_SHA256`,
`pinned_sha256`). Tests decode every edition and profile to the content
fingerprints taken from the format it replaced.

## Runtime catalogs

With the `runtime` feature, and without `embedded` if the binary should
not carry the container, a host installs an edition from snapshot bytes
it loads itself; the bytes must match the edition's pin or the
container's:

```rust
use ifc_template_catalog::catalog::CatalogProfile;
use ifc_template_catalog::definition::CatalogEdition;
use ifc_template_catalog::runtime;

let edition = CatalogEdition::Ifc4x3Add2;
runtime::install(edition, &std::fs::read("catalog/ifc4x3-add2.bin")?)?;
let catalog = runtime::load_catalog(edition, CatalogProfile::Corrected)?;
```

Write the per-edition files with
`cargo run -p ifc-template-catalog --features runtime --example export_snapshots -- <dir>`;
it checks each against its pin. The npm package `@openbim/ifc` ships them
and loads them this way.

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
