# ifc-template-catalog instructions

Purpose: Versioned external PSD/QTO definitions, applicability lookup, provenance, and explicit correction overlays.

Follow `../AGENTS.md`. Read sibling `PLAN.md` only when assigned implementation, generation, or review; record WIP, blockers, and proof there, not here.

## Boundary

This metadata-tier crate owns no IFC model records. It may use `ifc-schema` for adapters, but never depends on `ifc-model`, codecs, geometry, or `ifc-properties`.

## Module map

- `definition` - typed set, property, quantity, applicability, and provenance values.
- `catalog` - immutable indexed snapshots and profile selection.
- `archive` / `embedded` - versioned committed artifact decode and cached loaders.
- `query` / `diagnostic` - applicability lookup and source/schema checks.
- `compliance` - format-neutral application sink and authored-set validation.
- `overlay` - declarative corrections, advisories, and conflict detection.
- `xml` - optional bounded PSD/QTO import used by deterministic generation.

## Invariants

- Official source data is immutable; fixes are ledger entries with IDs and evidence.
- Preserve raw applicability beside normalized selectors.
- No network or `references/` access during build or runtime.
- Unknown XML semantics fail explicitly; do not silently discard typed content.
- Public snapshots are immutable. Create a new snapshot when sources or overlays change.

## Verification

Run crate tests, clippy/docs, generation drift checks, isolated facade feature builds, and IFC architecture/progressive-context gates. Exact WIP commands and results belong in `PLAN.md`.
