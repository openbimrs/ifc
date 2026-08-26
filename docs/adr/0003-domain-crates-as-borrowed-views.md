# 0003 — Domain crates as borrowed views

- **Status:** Accepted
- **Date:** 2026-08-26
- **Deciders:** openbimrs contributors
- **Supersedes:** —

## Context

Given an untyped entity graph ([ADR 0001](/adr/0001-entity-graph-free-of-domain-and-codec)),
domain meaning has to live somewhere. The question is whether a domain crate
*owns* data or *interprets* data.

Ownership is the intuitive choice — parse the model into `Vec<CostItem>` and
work with those. It also destroys the round-trip property: the owned copy is now
the source of truth, the original entities are discarded, and anything the
domain type failed to capture is lost on write.

A second force: IFC resource names are not clean module boundaries. The schema
mixes storage, geometry input, presentation, and domain semantics, and a single
entity can carry meaning in more than one dimension. `IfcWall` has a domain
identity, a geometric representation, material associations, and presentation
styling. Partitioning crates by IFC schema name would put unrelated concerns in
one crate and split related ones apart.

## Decision

We will implement domain crates as **borrowed views**: structures holding
`&Model` plus an `EntityId`, computing interpretations on access, owning no
copied data.

We will partition crates by **role in the pipeline**, not by IFC schema name.
One IFC entity may therefore have projections in two crates; `ifc-model` owns
the record and neither projection duplicates it.

## Alternatives considered

| Option | Why not |
| --- | --- |
| Domain crates own parsed copies | Breaks round-tripping; two sources of truth; memory doubled |
| One crate per IFC schema | Schema mixes concerns; produces incoherent crates and cross-crate cycles |
| One monolithic domain crate | Every consumer compiles every domain; no feature granularity |

## Consequences

**Positive**

- The round-trip property survives, because views never replace the record.
- Domains are independently optional cargo features.
- A view is cheap to construct — no parse pass, no allocation of a parallel tree.
- Competing interpretations can coexist as separate crates.

**Negative / costs**

- Views carry a lifetime, which propagates into calling code.
- Interpretation cost is paid per access rather than once, so hot loops must
  hoist their own caching.
- Sibling domain crates may not depend on one another, so cross-domain workflows
  must live in an orchestration layer above them.

**Follow-ups / risks to watch**

- Geometry-derived values such as area and volume must be computed outside the
  semantic crate and written through it by an application service, or the
  layering inverts.

## Relation to existing code

- `ifc-cost` is the reference implementation of the pattern (`view.rs`,
  `rollup.rs`, `item.rs`)
- `ifc-geometry/src/slots.rs` is the borrowed-slot-access house pattern
- Dependency tiers are documented in the repository **AGENTS.md**
- `openbim-ifc/tests/costing_roundtrip.rs`
