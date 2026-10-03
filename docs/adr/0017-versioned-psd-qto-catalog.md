# 0017 — Versioned PSD/QTO template catalogs

- **Status:** Accepted
- **Date:** 2026-08-19
- **Deciders:** openbimrs contributors
- **Supersedes:** —

> Restored on 2026-09-26. This decision was accepted on 2026-08-19 as ADR 0010
> of the pre-split repository and lost when the IFC family moved into its own
> repository, where 0010 was reassigned. `ifc-template-catalog` implements it
> and cites it. The text is the original; only the paths below are updated to
> this repository's layout.

## Context

IFC publishes PSD and QTO XML catalogs outside authored IFC models. They define expected set names, property or quantity types, and entity/predefined-type applicability. The catalogs are versioned publication data, contain known defects, and are not the same concern as borrowed views over authored `IfcPropertySetDefinition` records.

Applications need an interoperable upstream view and an explicitly corrected view without hidden mutation of source data. Remote PSD/QTO schema locations are unreliable, and reference checkouts cannot be build or runtime dependencies.

The research note it cited did not move with the family.

## Decision

We will implement external PSD/QTO data in a separate `ifc-template-catalog` crate.

- `ifc-properties` owns authored property, quantity, unit, and in-model template projections.
- `ifc-template-catalog` owns versioned external catalogs, import, lookup, provenance, diagnostics, and declarative corrections.
- Official snapshots are immutable generated artifacts with precise release identity and source checksums.
- `Official` preserves normalized upstream semantics. `Corrected` applies an ordered, auditable patch ledger. Custom overlays are explicit and conflict checked.
- Applicability retains the raw source and exposes structured entity/predefined-type selectors. Subtype matching is supplied by schema metadata rather than hard-coded into catalog data.
- Standard builds do not read `references/`, parse XML, or access the network.
- EPD lifecycle-module semantics belong in the separate `openbimrs/epd` family, not in this generic catalog.

## Alternatives considered

| Option | Why not |
| --- | --- |
| Put generated catalogs in `ifc-properties::standard` | Couples authored instance semantics to optional bulk data and correction policy. |
| Modify imported rows directly | Loses reproducibility and makes upstream comparison ambiguous. |
| Runtime-only XML loading | Adds avoidable startup/dependency cost and relies on broken remote schema locations. |
| Copy IfcOpenShell template IFC files | Hides corrections behind ordering and imports another project's generated artifact contract. |
| Treat EPD remodeling as a catalog correction | The replacement is not isomorphic to the legacy environmental Psets. |

## Consequences

**Positive**

- Thin IFC/property builds do not carry standard catalog data.
- Upstream and corrected behavior are selectable and explainable.
- Generation and runtime lookup are independently testable.

**Negative / costs**

- A new crate, generated artifact, importer, and patch format must be maintained.
- Applications compose catalog definitions with authored property APIs explicitly.

**Follow-ups / risks to watch**

- Measure generated-code compile time, binary size, load time, and lookup latency.
- Keep copied descriptive text out of MIT artifacts until redistribution terms are resolved.
- Add IFC2X3 and IFC4X3 only from pinned, checksummed release inputs.

## Relation to existing code

- `ifc-template-catalog/`
- `ifc-properties/`
- `openbim-ifc/`
- `ifc-model/tests/package_architecture.rs`

## Amendments

- *Amended 2026-10-03 (#317, #318): snapshot format 3 and runtime
  catalogs.* The three bincode snapshots (3.65 MB) are replaced by one
  container, `data/catalog.bin` (1.42 MB), holding every edition. The
  change is a lossless format shift and nothing else: every set, member,
  definition, translation, applicability selector and provenance field
  decodes as before, which a test asserted against the old snapshots, for
  every edition and both profiles, before they were removed; content
  fingerprints taken from them stay pinned. Strings used more than once are
  stored once in a table, strings used once stay inline, and each distinct
  record is stored once across editions; each edition keeps its own
  manifest, source digest and per-template provenance, so release identity
  stays exact and nothing is shared by name. The container is not
  compressed on top: no decompressor in every binary, and every channel the
  crate ships through (crates.io, wheels, npm, HTTP) compresses already.
  The generator maintains the container deterministically, and the SHA-256
  of the container and of each per-edition file written from it is pinned
  in the crate. A host that should not embed the catalog (the npm package)
  installs an edition at runtime from its per-edition file (feature
  `runtime`), and bytes that do not match the pin are refused. Standard
  builds still read neither XML nor `references/`, and the network only
  when a host fetches a per-edition file it ships itself.

