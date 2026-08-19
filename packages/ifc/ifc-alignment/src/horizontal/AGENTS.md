# ifc-alignment horizontal instructions

Scope: horizontal layout and exact horizontal segment parameters. Follow the crate `../../AGENTS.md`. Read `PLAN.md` only for
assigned task(s) `ALIGN-H`; keep progress and blockers there.

## Owns

- segment order/transition
- line/arc/spiral parameter views
- horizontal continuity diagnostics

## Does not own

- polyline approximation
- vertical/cant projection
- road design policy

## Growth map

`layout.rs`, `segment.rs`, `transition.rs`. These source owners already compile as private scaffold modules. Replace a module's planned-owner marker with its first real contract and tests; do not add parallel placeholders. Keep views, resolution, validation, and neutral output in
separate files.
