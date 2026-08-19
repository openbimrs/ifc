# ifc-style assignment instructions

Scope: style and presentation-layer associations to representation EntityIds. Follow the crate `../../AGENTS.md`. Read `PLAN.md` only for
assigned task(s) `STYLE-ASSIGN`; keep progress and blockers there.

## Owns

- IfcStyledItem and style select links
- layer assignment links
- deterministic association lookup

## Does not own

- geometry-node imports
- renderer material creation
- texture loading

## Growth map

`styled_item.rs`, `layer.rs`, `resolution.rs`. These source owners already compile as private scaffold modules. Replace a module's planned-owner marker with its first real contract and tests; do not add parallel placeholders. Keep views, resolution, validation, and neutral output in
separate files.
