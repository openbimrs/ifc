# 0002 — Codec as a trait owned by the model crate

- **Status:** Accepted
- **Date:** 2026-08-26
- **Deciders:** openbimrs contributors
- **Supersedes:** —

## Context

IFC has several encodings of the same information model: the ISO 10303-21 STEP
physical file (`.ifc`), ifcXML, and an emerging IFC-JSON. They differ in syntax,
not in the graph they describe.

The naive structure makes the STEP parser own the model type, because STEP came
first and is dominant. Every other encoding then either depends on the STEP
crate for its type definitions — dragging a parser it never uses into the build
— or defines a parallel model, making format conversion a translation between
two in-memory representations rather than a read and a write.

## Decision

We will define the read/write contract as a trait, `Codec`, **in the model
crate**, and have each encoding implement it in its own crate.

Consequently format conversion is: read with one implementation, write with
another. No conversion code exists, because none is needed.

## Alternatives considered

| Option | Why not |
| --- | --- |
| Model type owned by the STEP crate | Every consumer pulls in a STEP parser; ifcXML-only builds carry dead code |
| A trait in a separate `ifc-codec` crate | An extra crate whose only content is one trait definition that the model must depend on anyway |
| Enum of known formats in the model | Adding a format means editing the model crate; third parties cannot add one out of tree |

## Consequences

**Positive**

- Adding IFC-JSON requires no change to `ifc-model` and no change to existing
  codecs — it is a new crate implementing an existing trait.
- Codecs are cargo features; a viewer that only reads `.ifc` compiles no XML
  machinery.
- Format conversion is trivially correct because both sides share one graph.
- A third party can implement a private encoding without forking.

**Negative / costs**

- The trait must stay encoding-neutral. Any STEP-specific concept that leaks
  into its signature would break ifcXML or IFC-JSON implementations.
- Header handling differs meaningfully between encodings and must be modelled
  generally enough to serve all of them.

**Follow-ups / risks to watch**

- Watch for pressure to add encoding-specific options to the trait. Those belong
  on the concrete codec type, not the shared contract.

## Relation to existing code

- `ifc-model/src/codec.rs` defines the trait
- `ifc-step` and `ifc-xml` implement it; neither depends on the other
- `ifc-step/tests/roundtrip.rs`, `ifc-xml/tests/roundtrip.rs`
- Generic ISO 10303-21 syntax is further delegated to the `openbim-step` crate,
  leaving `ifc-step` as a thin IFC-specific adapter
