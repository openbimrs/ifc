# ifc-alignment placement instructions

Scope: linear placement and point-by-distance interpretation. Follow the crate `../../AGENTS.md`. Read `PLAN.md` only for
assigned task(s) `ALIGN-PLACE`; keep progress and blockers there.

## Owns

- linear referencing values
- point-by-distance/select resolution
- station/frame output

## Does not own

- geodetic conversion
- generic product placement ownership
- unbounded recursion

## Growth map

`linear.rs`, `distance.rs`, `station.rs`. These source owners already compile as private scaffold modules. Replace a module's planned-owner marker with its first real contract and tests; do not add parallel placeholders. Keep views, resolution, validation, and neutral output in
separate files.
