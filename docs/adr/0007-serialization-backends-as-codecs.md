# 0007 — Serialization backends as pluggable codecs

- **Status:** Accepted
- **Date:** 2026-08-18
- **Deciders:** GeneralPawz, Hermes
- **Extends:** 0006

## Context

ADR 0006 established that `ifc-model` knows no serialization: `Codec` is a
trait *in the model crate*, and `ifc-step` implements it. That was one codec, so
the claim "serialization is pluggable" was structurally plausible but
unproven — a single implementation always fits its own abstraction.

IFC has at least three encodings of the same data model: STEP physical file
(ISO 10303-21), ifcXML (ISO 10303-28), and prospectively IFC-JSON.

## Decision

Add `ifc-xml` as a second codec and select codecs by cargo feature on the `ifc`
facade (`step`, `ifcxml`). The model crate did **not** change to accommodate
the second codec, which is the evidence the abstraction is real.

Two supporting decisions fell out of implementing it:

### 1. The schema is optional to the XML codec

STEP records are positional (`#5=IFCWALL('guid',#1,$)`); ifcXML is named
(`<IfcWall id="i5" GlobalId="guid"/>`). Crossing between them requires the
schema to know that slot 0 is `GlobalId`.

Making the schema a hard dependency would mean a file whose schema we lack
cannot be written at all. So `ifc-xml` has a `schema` feature:

| Build | Attribute names | Round-trips? | Interoperable? |
| --- | --- | --- | --- |
| with `schema` | `GlobalId`, `Coordinates` | yes | yes |
| without | `a0`, `a1`, ... | yes | no |

The fallback is deliberately an obvious placeholder rather than a guess. A
wrong name that looks plausible is worse than one that is visibly positional.

### 2. Value *kind* is preserved structurally, not by inference

XML attribute values are all strings. STEP distinguishes `$` (unset) from `*`
(derived), `.T.` from `.U.`, `1.` (real) from `1` (integer), and `'0.1'`
(string) from `0.1` (real). A codec that writes everything as an attribute and
re-infers types on read loses these distinctions.

Rule: only values whose kind is unambiguously recoverable become XML
attributes. Everything else becomes a typed child element carrying an explicit
`kind`.

This was not theoretical. `IfcApplication.Version` is commonly the string
`"0.1"`; written as a plain attribute it came back as `Real(0.1)`. The bug was
caught by a realistic fixture, not by a unit test, which is why the fixture
contains a full project header rather than only cost entities.

## Consequences

**Good**
- The pluggability claim is now demonstrated, not asserted. A third codec is
  additive.
- `cargo tree -p ifc --features step,ifcxml` resolves to model + schema + the
  two codecs. No domain crate, no geometry.
- The `detect` method on `Codec` (defaulting to `false`) lets `ifc::read_path`
  choose a codec by content, so a mis-named file still opens.

**Costs**
- Two encodings of the same model must be kept in agreement. The mitigation is
  a cross-codec test: STEP → XML → STEP must reproduce the entity graph
  exactly, so a divergence fails the build rather than corrupting a file.
- `ifc-xml`'s writer and reader are a matched pair: `writer::looks_numeric`
  must mirror `reader::infer_scalar`. This coupling is documented in both
  functions. If a third codec appears, extracting a shared "unambiguous scalar"
  helper is the right move.
- Round-trip fidelity is **semantic, not byte-identical**. Real lexemes
  normalize and comments are dropped. Tests compare entity graphs, and the
  claim is stated that way rather than overclaimed.

## Verification

Mutation-verified: five deliberate defects each fail
`ifc/tests/costing_roundtrip.rs`, and all restore green.

| Mutation | Caught |
| --- | --- |
| STEP writer skips unknown entity types | yes |
| STEP writer truncates reals to integers | yes |
| STEP writer collapses `.U.` into `.F.` | yes |
| XML writer loses the enum kind | yes |
| XML writer emits numeric-looking strings raw | yes |
