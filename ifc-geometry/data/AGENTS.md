# generated geometry schema data

Purpose: Committed, deterministic artifacts derived from the IFC4 ADD2 TC1
EXPRESS schema, plus this repository's implementation-ownership mapping.

Follow `../AGENTS.md`. Read `NOTICE.md` for provenance and licensing. Read
sibling `PLAN.md` only for regeneration, provenance, or licensing-gate work.

## Boundary

`declaration_manifest.rs` `include_str!`s the two TSVs, so the coverage audit
runs in a fresh clone with no local schema checkout. Nothing here reads
`references/`, XML, or the network at build time.

Never name this directory `references/`: `scripts/check-leakage.py` rejects
that name in the published tree, and it is where unredistributable standards
payloads live. Derived artifacts go in `data/`.

## Invariants

- Filenames record the exact IFC edition.
- `NOTICE.md` states source, license, and why redistribution is permitted.
- No EXPRESS source text -- structural facts only.
- Regeneration needs the local schema checkout and is a maintenance task,
  not a build step.
