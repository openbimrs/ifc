# 0001 — Entity graph free of domain semantics and serialization

- **Status:** Accepted
- **Date:** 2026-08-26
- **Deciders:** openbimrs contributors
- **Supersedes:** —

## Context

An IFC library has to serve populations with incompatible needs. A web viewer
wants to parse a 300 MB file and draw it, and cares nothing for cost or
scheduling. A take-off tool cares about quantities and not at all about
tessellation. A validator cares about neither.

Two failure modes are common in this space:

1. **Generated typed entities for the whole schema.** IFC4 ADD2 TC1 declares
   over 800 entities. Generating a struct per entity produces an enormous
   compile-time and binary-size cost for every consumer, whatever they actually
   use, and a new schema version means regenerating everything.
2. **Domain types as the storage model.** If `IfcCostItem` is stored as a
   `CostItem` struct, then a build without the cost module either cannot read
   the file or silently drops the data on write. Round-tripping a file you do
   not fully understand becomes impossible, which is the single most damaging
   property a BIM tool can have — it corrupts other people's data.

## Decision

We will store IFC as an untyped entity graph. `Model` holds
`(EntityId, type_name, Vec<Value>)` and knows nothing about what any type means,
nor about how it is serialized.

Domain meaning is supplied by separate crates that borrow `&Model` and
interpret it. Serialization is supplied by separate crates implementing a trait.

## Alternatives considered

| Option | Why not |
| --- | --- |
| Code-generate a struct per schema entity | Compile cost and binary size borne by every consumer; schema churn regenerates the world; unknown data still needs a fallback path |
| Domain structs as the storage model | Data the build does not understand cannot round-trip; loses information silently on write |
| Dynamic map of named attributes | IFC is positionally encoded; names come from the schema, so storing names duplicates schema data in every entity |

## Consequences

**Positive**

- Data the build does not understand round-trips byte-faithfully. A binary with
  no cost crate compiled reads, holds, and re-emits cost entities intact.
- A thin consumer compiles the model plus one codec and nothing else.
- Adding a domain interpretation is an additive crate, not a change to storage.
- Two different interpretations of the same entities can coexist.

**Negative / costs**

- Attribute access is positional, so callers use index constants or view types.
  Getting an index wrong is a runtime error rather than a compile error.
- No type-level guarantee that an entity is well-formed for its declared type;
  validation is a separate concern (`ifc-validate`).
- Authoring is harder than with typed constructors — see the roadmap item for a
  schema-checked authoring layer, which is the accepted cost of this decision.

**Follow-ups / risks to watch**

- The positional-access ergonomics gap is real and must be closed by a typed
  authoring/accessor layer built *over* the graph, never by changing storage.

## Relation to existing code

- `ifc-model/src/model.rs`, `entity.rs`, `value.rs`
- `openbim-ifc/tests/costing_roundtrip.rs` proves the round-trip property with
  no domain crate compiled
- `openbim-ifc/tests/thin_build.rs` guards the thin-consumer property
