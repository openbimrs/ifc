# 0020 — IFC5 is a separate family; no IFC-JSON codec

- **Status:** Accepted
- **Date:** 2026-10-03
- **Deciders:** openbimrs contributors
- **Supersedes:** —

## Context

This repository models the EXPRESS releases of IFC (IFC2X3 to IFC4X3): one
entity graph, schema tables generated from the `.exp` files, and two codecs
for that graph, STEP and ifcXML. Two questions about other formats were open:

- **IFC5 / IFCX** (#35). It is not another EXPRESS release. It is a
  layered, componentised data model, serialised as JSON and composed in the
  manner of USD. Its records do not map onto positional EXPRESS attributes,
  and its layering has no counterpart in `ifc-model`.
- **IFC-JSON** (#122): a third `Codec` for the existing graph, as in
  buildingSMART's ifcJSON work. That work never became a published
  buildingSMART standard, so a codec would fix this repository to a draft.

Every domain crate refuses the `IFC5` schema token today with a typed
`UnsupportedSchema` error. Until now that was only visible in tests.

## Decision

We will not support IFC5 in this repository. IFC5 is developed as its own
standard family in `openbimrs/ifcx`. We will not add an IFC-JSON codec.

- `SchemaVersion` stays limited to EXPRESS releases. The `IFC5` token remains
  a typed refusal, not an unknown-schema fallback.
- The family dependency rule holds: `openbimrs/ifcx` may depend on `step`,
  `core` or `ifc` (for example, for an IFC4 ↔ IFCX bridge), and `ifc` never
  depends on `ifcx`.
- The capabilities page lists both formats as absent and names this ADR, so
  the gap is stated rather than implied.

## Alternatives considered

| Option | Why not |
| --- | --- |
| IFC5 as another `SchemaVersion` | Forces a non-EXPRESS, layered model into a positional record graph; every domain view would need a second code path. |
| An `ifc5` crate inside this workspace | Different data model, schema source and release cadence; it would share little beyond the name and would bind both to one release train. |
| An IFC-JSON codec for the EXPRESS graph | No settled standard to conform to; JSON interchange with a future is IFCX, which `openbimrs/ifcx` owns. |

## Consequences

**Positive**

- The EXPRESS model and its codecs stay one coherent design.
- IFC5 work can follow the evolving buildingSMART drafts without a semver
  impact on the IFC crates.

**Negative / costs**

- A consumer needing both reads them through two families. A bridge, if one
  is needed, lives in `ifcx`.

**Follow-ups / risks to watch**

- If buildingSMART publishes a normative JSON encoding of the EXPRESS
  releases, revisit the IFC-JSON half of this decision.

## Relation to existing code

- `docs/capabilities.md` (IFC-JSON and IFC5 rows, explicit non-goals)
- `UnsupportedSchema` refusals of `IFC5` in `ifc-alignment`, `ifc-georef` and
  `ifc-material` tests
- `openbimrs/ifcx` (issue ifcx#1, formerly ifc#35)
