# openbim implementation plan

Status: name reserved; implementation not started.
Last updated: 2026-08-24

This is task state, not ambient context. Follow `AGENTS.md`; claim one task ID,
record blockers/decisions under it, and check it off only with evidence.

## Established boundary

Re-exports only. Never define a type here -- consumers may also depend on the leaf crate directly.

## Open work

See `docs/ROADMAP.md` Stage 5 for sequencing. Nothing is claimed here yet.

## Planned file map

These paths are compiled private scaffold modules. Implement inside the named
owner and expose a public symbol only through an intentional parent re-export.

- (none claimed yet)

## Work queue

- [ ] `OBF-FEAT` - add one feature per standard as each leaf crate gains a codec

## Completion log

Nothing completed yet. Record the proof command and its result here when an
item above is checked off.
