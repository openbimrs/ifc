# generated catalog data

Purpose: Committed, deterministic runtime artifacts generated from attributed upstream PSD/QTO catalogs.

Follow `../AGENTS.md`. Read sibling `PLAN.md` only for corpus generation, provenance, licensing, or reproducibility work.

## Boundary

Normal builds read only committed artifacts. They never access `references/`, XML parsers, the network, or upstream XSD locations.

## Invariants

- Generated filenames include the exact IFC edition.
- `NOTICE.md` records source URL, release, normalization, license, and digest.
- Regeneration is deterministic and fails count/type/import validation before writing.
- Official artifacts are immutable inputs; corrected profiles are runtime overlays.
