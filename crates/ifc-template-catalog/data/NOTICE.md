# IFC template-catalog provenance

The committed snapshot container (`catalog.bin`), the per-edition snapshot files written from it, and the TSV exports are deterministic format shifts of authenticated, read-only buildingSMART PSD/QTO XML. Normal builds require neither XML, network access, nor the reference checkout. Official and corrected profiles remain distinct: IFC4 ADD2 TC1 and IFC4X3 ADD2 have built-in corrected overlays; IFC2X3 TC1 exposes the unmodified official profile only. The IFC4X3 ADD2 overlay adds `Pset_Stationing.HasIncreasingStation` (`IfcBoolean`), listed by the published IFC 4.3.2.0 documentation (6.6.4.10) but absent from the `reference_schemas/psd` XML at the pinned commit; the official snapshot keeps the XML as published.

| Edition | Authenticated input | Inventory (PSD/QTO/members) | Ordered source SHA-256 | Per-edition snapshot (bytes / SHA-256) | TSV (rows / bytes / SHA-256) |
|---|---|---:|---|---|---|
| IFC2X3 TC1 | `ifc2x3-tc1/psd/psd` recursively; PSD-only (no standardized QTO XML supplied) | 317 / 0 / 1,856 / 0 | `395dcd1e8c6f8e5feeece08e8b46e211a3c35a7fc13a7a237485ea48b3c93d53` | 325,739 / `b416694e2ddcb0d3011a7acb1b23def34ae16b3f2d27156be74c594d42248955` | 3,019 / 1,029,873 / `6950f7686b67b68d456e2dde0be9fcae83cbc7849171a1d2c9bf95d4b0718586` |
| IFC4 ADD2 TC1 | `ifc4-add2-tc1/html/{psd,qto}` | 420 / 93 / 2,550 / 257 | `57227d4c82f9903bc59cb5bade18a49f2c5f2c9363d0293ccb68fed8765d36e3` | 1,015,315 / `1ea624205c45cb31489f265c53fcef1334ba8556046cf296f73ee4e7550c6539` | 3,525 / 1,280,455 / `15dca1204b3f7533b2ee85fe353ad1d9b23fdf318fcb46100bef45dd5c2eb42c` |
| IFC4X3 ADD2 | buildingSMART `IFC4.x-development` commit `524daac53ca682e0649d240ace87f4cd7baff6e7`, tree `5ac02c6686df303a49e9bf5c05c75a0c91240aa7`; `reference_schemas/psd` (PSD and QTO roots) | 502 / 110 / 2,918 / 324 | `b2f327638a844c8666d38dff90c5a48e12fdcec73da9efc4789e2dedd9239298` | 1,086,526 / `0cfa3e0700b98e65ad1c2743e92f65299246a4aaf663520837346196179a29c2` | 4,361 / 1,573,278 / `11fb5c50dd87b78ccc3f5c09470942cc5d454760d07054d9b879fafe26a737c1` |

All three editions are committed in one container, `catalog.bin` (snapshot format 3): 1,423,604 bytes, SHA-256 `875a3902ac9a5c9d1f3b21dcbd5afe27fc65d1363be4452e852f9c6815c75996`. It stores each distinct record and each repeated string once across editions; each edition keeps its own manifest, source digest and per-template provenance. The per-edition files above are not committed: `cargo run -p ifc-template-catalog --features runtime --example export_snapshots -- <dir>` writes them from the container and checks each SHA-256, and the npm package ships them. Until 2026-10-03 each edition was a separate bincode snapshot (format 2: 431,961, 1,537,256 and 1,679,356 bytes); format 3 decodes to exactly the same catalogs, which a test proved before they were replaced (#317).

The IFC4X3 source publishes `Pset_MarineVehicleCommon` as a `PropertySetDef`
with `QTO_TYPEDRIVENOVERRIDE`; the official profile retains its resulting
type-driven-override classification and applies no correction.

The source digest hashes sorted normalized relative source paths followed by bytes, with NUL separators, so rename-only changes are detectable. Per-template source paths and SHA-256 digests are carried in each snapshot and TSV row. The TSV also carries source-published set/member GUIDs and leaves absent GUIDs empty. Consumers must keep release membership explicit and must not infer cross-release identity from a shared name or GUID. The generator validates the exact release-specific PSD/QTO and typed-member counts before atomically replacing output, and rejects unsupported XML roots and typed property/quantity forms.

Regenerate one edition of the container explicitly (the other editions are kept as they are):

```bash
cargo run -p ifc-template-catalog --features generation --bin ifc-template-catalog-generate -- \
  <ifc2x3-tc1|ifc4-add2-tc1|ifc4x3-add2> <source-directory> [catalog.bin]
```

Generate a TSV explicitly by edition (the `edition` and `source_digest` columns make it suitable for version-indexed Pkl ingestion):

```bash
cargo run -p ifc-template-catalog --example export_ifc4_tsv -- \
  <ifc2x3-tc1|ifc4-add2-tc1|ifc4x3-add2> <output.tsv>
```

Upstream names, descriptions, aliases, GUIDs, applicability, units, and type declarations are copyright buildingSMART International Limited and published under CC BY-ND 4.0: https://technical.buildingsmart.org/standards/ifc/ifc-schema-specifications/

The snapshots and TSVs are deterministic format shifts without semantic edits. Crate code and Nehirde correction overlays are licensed under AGPL-3.0-or-later and remain separate; overlays never rewrite official artifacts. Redistribution must preserve this attribution and the upstream license.
