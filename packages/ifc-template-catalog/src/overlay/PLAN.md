# overlay implementation plan

Status: implemented and verified.

Follow `AGENTS.md`; this file owns transient overlay tasks and proof.

## Work queue

- [x] `OVL-CONTRACT` - patch, operation, advisory, applied-provenance types.
- [x] `OVL-APPLY` - immutable application with edition, target, stale, and duplicate checks.
- [x] `OVL-BUILTIN` - evidence-backed IFC4 correction/advisory ledger.
- [x] `OVL-VERIFY` - official immutability and deliberate mutation tests.

## Completion log

Append concise task proof; no full logs.

- Overlay tests cover immutable snapshots, stale targets, duplicate IDs, provenance, advisories, and official-data preservation.
