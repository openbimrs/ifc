# overlay instructions

Purpose: Declarative catalog corrections and advisories with stale/conflict detection.

Follow `../../AGENTS.md`. Read sibling `PLAN.md` only for overlay implementation or review; keep WIP there.

## Boundary

Overlays create new immutable snapshots. They never mutate official templates or hide source provenance.

## Invariants

- Every patch has a stable ID, exact edition, target, rationale, and evidence.
- Structural patches fail when their expected target state is absent or already changed.
- Advisories describe disputed/non-isomorphic semantics without rewriting data.
- Duplicate IDs and conflicting operations fail deterministically.
