# ifc-template-catalog instructions

Purpose: Versioned external PSD/QTO definitions, applicability lookup, provenance, and explicit correction overlays.

Follow `../../AGENTS.md`.

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
- The design is ADR 0017. Do not enable `quick-xml` features here that
  `ifc-xml` does not want: Cargo unifies features across the workspace, and
  `quick-xml/encoding` once broke `ifc-xml` that way.

## Verification

Run crate tests, clippy/docs, generation drift checks, isolated facade feature builds, and IFC architecture/progressive-context gates. Exact WIP commands and results belong in the pull request.
