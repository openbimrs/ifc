# xml instructions

Purpose: Tolerant syntax ingestion with strict typed interpretation of PSD/QTO semantics.

Follow `../../AGENTS.md`. Read sibling `PLAN.md` only for importer/generator work; record WIP and corpus evidence there.

## Boundary

The XML reader may preserve publication quirks but must not invent IFC meaning. Generation is offline; normal embedded lookup does not require XML.

## Invariants

- Namespace prefixes do not affect local element interpretation.
- Every property/quantity type is consumed or reported as unsupported.
- Applicability keeps raw selectors and normalized entity/predefined-type fields.
- Errors identify the set/property path and unsupported element.
- Tests use first-party minimal fixtures; optional corpus sweeps use ignored references.
