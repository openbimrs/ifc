# generated catalog plan

Status: implemented and verified.

## Work queue

- [x] `DATA-IFC4` - import 420 PSD and 93 QTO files.
- [x] `DATA-DIGEST` - deterministic path-plus-content SHA-256 and format version.
- [x] `DATA-NOTICE` - source attribution, normalization, and artifact licensing note.
- [x] `DATA-REPRO` - regenerate twice and compare bytes.
- [x] `DATA-RUNTIME` - embedded load count and benchmark proof.
- [x] `DATA-IFC4-TSV` - deterministic typed applicability export for each PSD/QTO member and entity selector.

## Completion log

Append exact counts, digest, artifact size, and proof commands.

- Two late-review runs compared byte-identical at 1,537,256 bytes; embedded tests verify 420/93 sets, 2,550/257 members, set/property/quantity/constant aliases, all set classifications, and per-template SHA-256 provenance.
- `DATA-IFC4-TSV` - 3,525 rows reproduced byte-identically at 1,064,231 bytes (SHA-256 `659958e84edab2c932214a64dcc62d725cbbecafeb092bef77d76bd82f8ad724`); tests pin 420 PSD and 93 QTO sets and preserve QTO `Q_*` value types.
