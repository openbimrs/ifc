# ifc-alignment cant instructions

Scope: cant/superelevation layout and transition parameters. Follow the crate `../../AGENTS.md`.

## Owns

- cant segment order
- left/right rail/crossfall values
- cant continuity and units

## Does not own

- vehicle dynamics
- rail mesh generation
- horizontal curve ownership

## Growth map

`layout.rs`, `segment.rs`, `transition.rs`. These source owners already compile as private scaffold modules. Replace a module's planned-owner marker with its first real contract and tests; do not add parallel placeholders. Keep views, resolution, validation, and neutral output in
separate files.
