# 0009 — DERIVED attributes resolve through the parent context

- **Status:** Accepted
- **Date:** 2026-08-26
- **Deciders:** openbimrs contributors
- **Supersedes:** —

## Context

`IfcGeometricRepresentationSubContext` redeclares six inherited attributes as
DERIVED. Real exporters write them as `*`:

```text
IFCGEOMETRICREPRESENTATIONSUBCONTEXT('Body','Model',*,*,*,*,#1,$,.MODEL_VIEW.,$)
```

`*` is not `$`. `$` means "not set"; `*` means "this value lives on my parent".
A consumer that reads the slot directly gets the marker, and one that treats the
marker as absent silently loses the project's precision, coordinate dimension
and world coordinate system — the last of which places all geometry authored
into that sub-context.

`ifc-model` already distinguishes the two: `Value::Derived` and `Value::Null`
are separate variants and survive a round trip. Nothing above the model acted on
the distinction.

## Decision

Reading an inherited attribute of a representation context walks
`ParentContext` past every `*` until it finds a concrete value.

- `*` continues the walk.
- `$` or an absent slot stops it and reports "unset".
- The walk is bounded by `MAX_PARENT_DEPTH` (8). A file that chains or cycles
  contexts terminates and reports the value as unresolved.

The bound is the *only* termination mechanism. An earlier draft also carried a
visited-set; mutation testing showed removing it could not change any outcome,
because the depth bound already bounds the walk. A guard that cannot fail is a
claim the tests cannot check, so it was deleted and the bound is now tested at
both edges: a four-link chain must resolve, and a cycle must give up.

## Consequences

- A sub-context reports the same precision and placement its parent declares,
  which is what the schema means.
- `$` on a child is honoured as "unset" even when the parent holds a value, so
  inheritance never invents data the author declined to give.
- Resolution costs a short pointer walk per query rather than being cached.
  Contexts are few — single digits in a typical file — so this is not worth an
  index until profiling says otherwise.
- Consumers must call the model-taking accessors (`precision(&model)`) rather
  than reading slots. The slot constants remain public for callers that
  genuinely want the raw marker.

## Alternatives considered

| Option | Why not |
| --- | --- |
| Treat `*` as absent | The silent-loss failure this ADR exists to prevent |
| Flatten contexts at parse time | Mutates the graph; breaks the lossless round-trip guarantee in ADR 0001 |
| Cache resolved values in an index | Premature: a file has a handful of contexts, and an index is stale state to keep coherent |
