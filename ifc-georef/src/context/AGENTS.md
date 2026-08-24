# ifc-georef context instructions

Scope: source-context association and composition boundary. Follow the crate `../../AGENTS.md`. Read `PLAN.md` only for
assigned task(s) `GEOREF-CHAIN`; keep progress and blockers there.

## Owns

- coordinate operation source/target links
- contract for composing supplied project frame with map conversion

## Does not own

- owning ifc-geometry context logic
- reading product placement trees
- implicit composition order

## Growth map

`source.rs`, `chain.rs`. These source owners already compile as private scaffold modules. Replace a module's planned-owner marker with its first real contract and tests; do not add parallel placeholders. Keep views, resolution, validation, and neutral output in
separate files.
