# 0005 — Scaffold modules declare ownership without claiming capability

- **Status:** Accepted
- **Date:** 2026-08-26
- **Deciders:** openbimrs contributors
- **Supersedes:** —

## Context

The repository contains a large number of modules whose entire content is a doc
comment of the form:

```rust
//! Planned owner: widths/fonts/colours.
//!
//! Follow the nearest AGENTS.md and PLAN.md. Keep this module
//! crate-private until it owns a deliberate public contract.
```

These exist because the crate and module layout was designed up front, from the
schema and from the dependency tiers, rather than grown ad hoc. Reserving the
structure keeps later work from fighting the layout.

The danger is that a directory listing is indistinguishable from a capability
listing. A file named `ifc-style/src/curve_style/style.rs` reads as line-style
support. An LLM-driven coding agent, in particular, will infer capability from
module names and confidently generate calls to an API that does not exist. That
failure is expensive and silent.

## Decision

We will keep the reserved structure, and we will make the distinction between
**reserved** and **implemented** explicit and machine-checkable:

1. Every scaffold module states `Planned owner:` in its first doc line and stays
   crate-private until it owns a tested public contract.
2. A capability claim must point at executable behaviour with a test, not at a
   module path.
3. Published documentation carries a status vocabulary — Implemented, Partial,
   Scaffold, Absent — and per-crate stub ratios derived from the source.
4. Lowering coverage is held as **data** (`IMPLEMENTED` and `PLANNED` arrays)
   so it is testable, rather than being scraped from source text.
5. Unimplemented paths return a typed `Unsupported` error naming the entity and
   the concrete missing capability — never a panic, never substituted geometry.

## Alternatives considered

| Option | Why not |
| --- | --- |
| Delete the scaffolds, add modules as implemented | Loses the designed layout; each addition re-litigates placement; **PLAN.md** ownership records are lost |
| Leave scaffolds undocumented | Readers and agents infer capability from names; produces confident wrong code |
| `todo!()` bodies instead of empty modules | Turns a documentation problem into a runtime panic; worse failure mode |

## Consequences

**Positive**

- The intended architecture is legible before it is built, and module ownership
  is settled in advance.
- A caller can determine what works from documentation and from typed errors,
  without reading every source file.
- Progress is auditable: implementing a family means moving a name from
  `PLANNED` to `IMPLEMENTED`, which the census test observes.

**Negative / costs**

- Stub ratios per crate are high, which looks like incompleteness on casual
  inspection. It *is* incompleteness — the decision is to be honest about it
  rather than hide it.
- The published capability matrix must be maintained alongside the code or it
  becomes exactly the misleading artefact it exists to prevent.

**Follow-ups / risks to watch**

- Docs drift is the main risk. Code examples in the documentation are compiled
  as a test, and the capability matrix is now machine-derived:
  `scripts/sync-capabilities.py` generates the workspace census, the
  representation-item table and the profile table from the source that
  implements them, and `scripts/gate.sh` fails when the page and the code
  disagree. This follow-up is closed.

## Relation to existing code

- `ifc-geometry/src/lower/dispatch.rs` — coverage as testable data
- `ifc-geometry/src/error.rs` — the typed `Unsupported` variant
- `ifc-model/tests/progressive_context.rs`, `required_scaffold_paths.txt`
- `openbim-ifc/tests/docs_examples.rs` — documented code compiled and asserted
- `docs/capabilities.md` — the published status vocabulary
