# 0007 — Authoring is a schema-layer concern, not a model-layer one

- **Status:** Accepted
- **Date:** 2026-08-26
- **Deciders:** openbimrs contributors
- **Supersedes:** —

## Context

`Model::push(Entity)` is the only construction primitive. It takes a type name
and a positional `Vec<Value>`, so an application that *generates* IFC must know
that `IfcAnnotation` has seven slots, that slot 0 is a 22-character GlobalId and
slot 2 is the name. Nothing checks any of it. The resulting file parses back
happily in this library and is rejected by other tools.

The obvious fix — "add typed setters to `Model`" — is not available. [ADR
0001](/adr/0001-entity-graph-free-of-domain-and-codec) makes the entity graph
free of domain semantics, and the dependency tiers place `ifc-model` at L0 with
`ifc-schema` above it at L1. A schema-checked builder inside `ifc-model` would
invert that edge and make the schema tables a mandatory dependency of every
consumer, including ones that only stream bytes.

The second obvious fix — generate a builder struct per entity type — is what
[ADR 0002](/adr/0002-codec-as-a-model-crate-trait) and the `ifc-schema` design
already rejected for readers: 776 types in IFC4, 876 in IFC4x3 with renames, and
a recompile to support a new schema release.

## Decision

Authoring lives in a new L2 crate, `ifc-author`, which depends on `ifc-model`
and `ifc-schema` and is reached through the facade's `author` feature.

It is **schema-driven, not generated**. A builder names attributes; the crate
resolves each name to its STEP slot via `Schema::attributes`, which already
returns inherited-first positional order. Arity, unknown attribute names,
required-but-missing slots, and declared-type mismatches are checked against the
same tables the readers use.

`ifc-model` gains no schema dependency and no typed setters. Its `mutation`
module remains the owner of schema-agnostic edits.

## Consequences

- A new schema release is a new `.exp` file, not a code generation run.
- Authoring is optional: `openbim-ifc` without `author` compiles no builder code.
- Validation is *construction-time*, so an invalid entity is never inserted —
  distinct from `ifc-validate`, which audits a model that already exists.
- The builder can only be as good as the schema tables. A type the schema does
  not declare cannot be checked; the crate reports that explicitly rather than
  silently accepting the entity.
- Applications that genuinely need to write an entity the schema does not know
  keep using `Model::push`, which stays public and unchecked.

## Alternatives considered

| Option | Why not |
| --- | --- |
| Typed setters on `Model` | Inverts the L0/L1 dependency; forces schema tables on every consumer |
| Generated struct per entity | 776–876 types per schema version, recompile per release; the problem `ifc-schema` exists to avoid |
| Builder inside `ifc-validate` | Conflates auditing an existing model with constructing a new one; different failure timing |
| No checking, document the slots | The status quo; every application reinvents an untested private version |
