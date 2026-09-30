# 0019 — Planned work is an issue, not a placeholder file

- **Status:** Accepted
- **Date:** 2026-09-29
- **Deciders:** openbimrs contributors
- **Supersedes:** 0005 (points 1 and 3: reserved `Planned owner:` modules and
  published stub counts)

## Context

ADR 0005 kept a designed module layout by reserving files whose only content
was a `//! Planned owner:` doc comment, and published a per-crate stub count so
the reservations would not read as capability. A month later 57 such files
remained. Most had been overtaken: the behaviour they reserved landed in
another module (`pset::table` inside `exact`, spatial containment in
`ifc-spatial`), so the name pointed at the wrong place. The rest named work
that had no issue, so nothing scheduled it. The stub count, measured as "files
of twelve lines or fewer", also counted small real modules, and crates marked
Implemented carried more placeholders than crates marked Partial. Readers of
the capabilities page reasonably took the count as a statement of what was
missing; it was not.

ADR 0016 already moved open work into issues. The placeholders were the last
place where planned work lived in the tree.

## Decision

We will record planned work only as issues. No source file consists of doc
comments alone.

1. Every `Planned owner:` module is deleted. A capability it reserved that the
   code still lacks becomes an issue; one that landed elsewhere is recorded in
   the deleting commit.
2. A module that holds only comments fails
   `ifc-model/tests/progressive_context.rs`.
3. A crate's status stays a declared judgement. A `partial` crate names the
   issues it lacks in `[package.metadata.openbim] gaps`, and the capabilities
   page links them instead of publishing a stub count.

Points 2, 4 and 5 of ADR 0005 stand: capability claims point at tested
behaviour, lowering coverage is data, and unimplemented paths return typed
errors.

## Alternatives considered

| Option | Why not |
| --- | --- |
| Keep the placeholders with a sharper stub count | The layout they reserved had already drifted from where the code lives; a precise count of misleading names is still misleading |
| Keep placeholders that name real gaps | Duplicates the issue, and a file cannot say who is working on it or why it waits |

## Consequences

**Positive**

- A module path in the tree is always backed by code.
- "Partial" carries its reason: the linked issues.

**Negative / costs**

- New work re-decides its module placement when it lands. The crate README's
  ownership section and the issue carry that decision now.

## Relation to existing code

- `crates/ifc-model/tests/progressive_context.rs`,
  `crates/ifc-model/tests/required_scaffold_paths.txt`
- `xtask/src/docs/capabilities/census.rs`, `xtask/src/workspace.rs`
- `docs/capabilities.md`
