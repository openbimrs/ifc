# catalog tooling instructions

Purpose: Deterministic offline import and generation of committed runtime artifacts.

Follow `../AGENTS.md`. Read sibling `PLAN.md` only for generator work; record corpus/import WIP there.

## Boundary

Tools may read explicit upstream source directories and enable XML/hash dependencies. Library runtime paths must not.

## Invariants

- Inputs are sorted by normalized relative path before hashing or parsing.
- Validate exact edition counts and typed child counts before atomic output replacement.
- Hash both relative paths and bytes to detect rename-only changes.
- Never encode local absolute paths or timestamps.
