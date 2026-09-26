# ifc-xml instructions

Purpose: ifcXML codec adapter between XML and ifc-model.

Follow `../AGENTS.md`.

## Boundary

Allowed production dependencies: ifc-model; ifc-schema is optional naming metadata; ifc-step is test-only differential evidence.

## Module ownership

- `codec.rs`: codec state, profile/schema constructors, and `Codec` adapter
- `reader.rs`: namespace-aware XML to Model and path context tracking
- `writer.rs`: deterministic Model to XML
- `scalar.rs`: the one scalar lexical contract, both directions: attribute
  inference, the writer's attribute/element choice (defined by calling that
  inference), and explicit `kind` element forms
- `slots.rs`: attribute name to positional slot, both directions (schema
  names or `a<i>`)
- `profile.rs`: explicit release namespace/schema-token contracts
- `error.rs`: typed XML failures and inspectable `XmlPath`
- `tests/namespaces.rs`: strict-profile and namespace-spoofing contracts
- `tests/diagnostics.rs`: nested entity/attribute/list error paths
- `tests/scalar_contract.rs`: seeded property test of the scalar contract
- `tests/corpus_roundtrip.rs`: STEP -> XML -> Model -> STEP over every
  committed fixture, with and without schema names

## Invariants

- No domain semantics in the codec.
- Schema-disabled mode must remain useful and must not fabricate schema names.
- Unknown data survives semantically even when XML lexical form normalizes.
- Compatibility mode must not be described as XSD conformance.
- Strict mode validates every resolved element namespace and the root profile token.
- Explicit typed-value errors retain entity/attribute paths; they never coerce to null.

Keep `lib.rs` delegating, keep child modules crate-private until they own a real
public contract, and split view/data, traversal, mutation, and validation before
they grow together.

## Verification

Run targeted crate tests and clippy first, then the package architecture/context
gates from `../AGENTS.md`. Record exact exit evidence in the pull request.
