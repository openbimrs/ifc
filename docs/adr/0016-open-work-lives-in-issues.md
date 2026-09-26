# 0016 — Open work lives in issues, not in checked-in plans

- **Status:** Accepted
- **Date:** 2026-09-26
- **Deciders:** openbimrs contributors
- **Supersedes:** — (amends the context protocol that ADR 0005's scaffold
  header refers to)

## Context

From the repository's first weeks, every crate and many modules carried a
`PLAN.md` beside their `AGENTS.md`: a checkbox work queue with task IDs, proof
commands and a completion log. It let an agent pick up a task with its proof
requirements in one place, while the project had no issue tracker in use.

By the time GitHub issues, a contributor project board and per-crate
changelogs existed, the plans had drifted. Of 95 open tasks across 76 files,
most described work that had already landed: every quantity task was open
while quantities shipped, and owner-history authoring was open while
`ifc-author/src/owner.rs` existed. Nothing checked a checkbox against the
code, so an unticked box read as a gap to anyone scoping work. The backlog also
existed twice, in the plans and in the issues, with no rule for which one won.

## Decision

We will keep open work in exactly one place: GitHub issues.

- No `PLAN.md` is tracked. `cargo run -p xtask -- plans --check` and
  `ifc-model/tests/progressive_context.rs` both fail if one returns.
- A marker of unfinished work in code names its issue: `TODO(#123)`.
  `cargo run -p xtask -- todo --check` fails the gate on a bare `TODO`,
  `FIXME`, `todo!()` or `unimplemented!()`, and `cargo run -p xtask -- todo`
  lists every marker with its issue so a closed issue's leftovers can be found.
- Proof of finished work goes in the pull request. A decision that must outlive
  it goes into the owning `AGENTS.md`, the module docs, or an ADR.
- An ownership scaffold (ADR 0005) is registered in
  `ifc-model/tests/required_scaffold_paths.txt` rather than in a plan's file
  map; its header points at its `AGENTS.md` only.
- The migration triaged every open plan task against the code: done tasks were
  dropped with their evidence recorded in the pull request, open ones became
  issues, and load-bearing decisions from completion logs moved into
  `AGENTS.md` files.

## Alternatives considered

| Option | Why not |
| --- | --- |
| Keep `PLAN.md` and add a checker | A checkbox cannot be checked against code in general; the checker would verify format, as the old one did, not truth. The duplicate backlog would remain. |
| Keep plans for design notes only | Design notes that matter are decisions, which already have homes (`AGENTS.md`, module docs, ADRs); the rest is history, which git keeps. |
| Generate plans from issues | Adds a sync job to keep a second copy of data the tracker already shows. |

## Consequences

**Positive**

- One backlog, with assignment, discussion and closure in the tool built for it.
- A code marker and its issue point at each other; closing the issue is the
  moment to delete the marker.
- Ambient context shrinks: an agent reads `AGENTS.md` files only.

**Negative / costs**

- Working offline, an agent cannot see the backlog; it sees the `TODO(#N)`
  markers and the `AGENTS.md` boundaries only.
- Task IDs such as `GEOM-CURVE` survive only in issue titles and git history.

**Follow-ups / risks to watch**

- A weekly job that lists markers whose issue is closed needs network access,
  so it runs outside the gate.

## Relation to existing code

- `xtask/src/plans.rs`, `xtask/src/todo.rs` and `scripts/gate.sh`.
- `ifc-model/tests/progressive_context.rs` and
  `ifc-model/tests/required_scaffold_paths.txt`.
- The root `AGENTS.md` context protocol, `CONTRIBUTING.md` and
  `.github/PULL_REQUEST_TEMPLATE.md`.
