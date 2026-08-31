# ifc-style assignment plan

Status: implemented under `STYLE-ASSIGN`. Last updated: 2026-08-31.
Follow `AGENTS.md`; record remaining scope without reopening completed tasks.

## Work queue

- [x] `SASSIGN-ITEM` - strict styled-item/select views
  - Proof: valid, malformed-select, wrong-type, and legacy-wrapper tests.
- [x] `SASSIGN-LAYER` - strict layer links
  - Proof: cross-schema empty-layer and wrong-member tests.
- [x] `SASSIGN-RESOLVE` - explicit precedence and ambiguity handling
  - Proof: direct-over-layer, ambiguity, and IFC2X3 wrapper-resolution tests.

## Completion log

- `SASSIGN-ITEM` - typed style members preserve wrapper discriminators; null style is never effective.
- `SASSIGN-LAYER` - layer members are validated without inventing a non-empty requirement.
- `SASSIGN-RESOLVE` - direct styles win deterministically while layer provenance remains observable.
