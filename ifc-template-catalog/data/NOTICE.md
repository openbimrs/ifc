# IFC template-catalog provenance

Committed binary snapshots and TSV exports are deterministic format shifts of authenticated, read-only buildingSMART PSD/QTO XML. Normal builds require neither XML, network access, nor the reference checkout. Official and corrected profiles remain distinct: only IFC4 ADD2 TC1 currently has built-in corrected overlays; IFC2X3 TC1 and IFC4X3 ADD2 expose the unmodified official profile only.

| Edition | Authenticated input | Inventory (PSD/QTO/members) | Ordered source SHA-256 | Binary (bytes / SHA-256) | TSV (rows / bytes / SHA-256) |
|---|---|---:|---|---|---|
| IFC2X3 TC1 | `ifc2x3-tc1/psd/psd` recursively; PSD-only (no standardized QTO XML supplied) | 317 / 0 / 1,856 / 0 | `395dcd1e8c6f8e5feeece08e8b46e211a3c35a7fc13a7a237485ea48b3c93d53` | 431,961 / `2fa03ce08e0ef3dd577f4961d183ae006c4a18644ea10d71b8cfdb0108ae3bc2` | 3,019 / 1,029,873 / `6950f7686b67b68d456e2dde0be9fcae83cbc7849171a1d2c9bf95d4b0718586` |
| IFC4 ADD2 TC1 | `ifc4-add2-tc1/html/{psd,qto}` | 420 / 93 / 2,550 / 257 | `57227d4c82f9903bc59cb5bade18a49f2c5f2c9363d0293ccb68fed8765d36e3` | 1,537,256 / `fe5567f0d30f8a4eb87a31bd34b8f43df95e2d28d72e7b56ffd082206bd48363` | 3,525 / 1,280,455 / `15dca1204b3f7533b2ee85fe353ad1d9b23fdf318fcb46100bef45dd5c2eb42c` |
| IFC4X3 ADD2 | buildingSMART `IFC4.x-development` commit `524daac53ca682e0649d240ace87f4cd7baff6e7`, tree `5ac02c6686df303a49e9bf5c05c75a0c91240aa7`; `reference_schemas/psd` (PSD and QTO roots) | 502 / 110 / 2,918 / 324 | `b2f327638a844c8666d38dff90c5a48e12fdcec73da9efc4789e2dedd9239298` | 1,679,356 / `61bf96f79b59166d98335885e38318b47d9b8a560c6e60495d62f43b53b5f8da` | 4,361 / 1,573,278 / `11fb5c50dd87b78ccc3f5c09470942cc5d454760d07054d9b879fafe26a737c1` |

The IFC4X3 source publishes `Pset_MarineVehicleCommon` as a `PropertySetDef`
with `QTO_TYPEDRIVENOVERRIDE`; the official profile retains its resulting
type-driven-override classification and applies no correction.

The source digest hashes sorted normalized relative source paths followed by bytes, with NUL separators, so rename-only changes are detectable. Per-template source paths and SHA-256 digests are carried in each binary and TSV row. The TSV also carries source-published set/member GUIDs and leaves absent GUIDs empty. Consumers must keep release membership explicit and must not infer cross-release identity from a shared name or GUID. The generator validates the exact release-specific PSD/QTO and typed-member counts before atomically replacing output, and rejects unsupported XML roots and typed property/quantity forms.

Generate a binary explicitly by edition:

```bash
cargo run -p ifc-template-catalog --features generation --bin ifc-template-catalog-generate -- \
  <ifc2x3-tc1|ifc4-add2-tc1|ifc4x3-add2> <source-directory> [output.bin]
```

Generate a TSV explicitly by edition (the `edition` and `source_digest` columns make it suitable for version-indexed Pkl ingestion):

```bash
cargo run -p ifc-template-catalog --example export_ifc4_tsv -- \
  <ifc2x3-tc1|ifc4-add2-tc1|ifc4x3-add2> <output.tsv>
```

Upstream names, descriptions, aliases, GUIDs, applicability, units, and type declarations are copyright buildingSMART International Limited and published under CC BY-ND 4.0: https://technical.buildingsmart.org/standards/ifc/ifc-schema-specifications/

The binaries and TSVs are deterministic format shifts without semantic edits. Crate code and Nehirde correction overlays are licensed under AGPL-3.0-or-later and remain separate; overlays never rewrite official artifacts. Redistribution must preserve this attribution and the upstream license.
