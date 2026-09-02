# generated catalog plan

Status: implemented and verified.

## Work queue

- [x] `DATA-IFC4` - import 420 PSD and 93 QTO files.
- [x] `DATA-DIGEST` - deterministic path-plus-content SHA-256 and format version.
- [x] `DATA-NOTICE` - source attribution, normalization, and artifact licensing note.
- [x] `DATA-REPRO` - regenerate twice and compare bytes.
- [x] `DATA-RUNTIME` - embedded load count and benchmark proof.
- [x] `DATA-IFC4-TSV` - deterministic typed applicability export for each PSD/QTO member and entity selector.
- [x] `DATA-EDITIONS` - authenticated IFC2X3/IFC4/IFC4X3 artifacts and
  release-scoped set/member GUID columns without inferring cross-release identity.

## Completion log

Append exact counts, digest, artifact size, and proof commands.

- Two late-review runs compared byte-identical at 1,537,256 bytes; embedded tests verify 420/93 sets, 2,550/257 members, set/property/quantity/constant aliases, all set classifications, and per-template SHA-256 provenance.
- `DATA-IFC4-TSV` - 3,525 rows reproduced byte-identically at 1,280,455 bytes (SHA-256 `15dca1204b3f7533b2ee85fe353ad1d9b23fdf318fcb46100bef45dd5c2eb42c`); tests pin 420 PSD and 93 QTO sets, source GUIDs, and QTO `Q_*` value types.
- `DATA-EDITIONS` - all three binaries and TSVs reproduced from the attributed
  corpora; `data/NOTICE.md` pins exact inputs, counts, sizes, and SHA-256 values.
