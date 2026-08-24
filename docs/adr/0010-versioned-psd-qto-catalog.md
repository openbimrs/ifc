# 0010 - Versioned PSD/QTO template catalogs

- **Status:** Accepted
- **Date:** 2026-08-19
- **Deciders:** Friedrich
- **Supersedes:** -

## Context

IFC publishes PSD and QTO XML catalogs outside authored IFC models. They define expected set names, property or quantity types, and entity/predefined-type applicability. The catalogs are versioned publication data, contain known defects, and are not the same concern as borrowed views over authored `IfcPropertySetDefinition` records.

Applications need an interoperable upstream view and an explicitly corrected view without hidden mutation of source data. Remote PSD/QTO schema locations are unreliable, and reference checkouts cannot be build or runtime dependencies.

Detailed evidence is in `../research/psd-qto-template-architecture.md`.

## Decision

We will implement external PSD/QTO data in a separate `ifc-template-catalog` crate.

- `ifc-properties` owns authored property, quantity, unit, and in-model template projections.
- `ifc-template-catalog` owns versioned external catalogs, import, lookup, provenance, diagnostics, and declarative corrections.
- Official snapshots are immutable generated artifacts with precise release identity and source checksums.
- `Official` preserves normalized upstream semantics. `Corrected` applies an ordered, auditable patch ledger. Custom overlays are explicit and conflict checked.
- Applicability retains the raw source and exposes structured entity/predefined-type selectors. Subtype matching is supplied by schema metadata rather than hard-coded into catalog data.
- Standard builds do not read `references/`, parse XML, or access the network.
- EPD lifecycle-module semantics belong in a later `packages/epd` package, not in this generic catalog.

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

- `packages/ifc-template-catalog/`
- `packages/ifc-properties/`
- `packages/ifc/`
- `packages/ifc-model/tests/package_architecture.rs`
