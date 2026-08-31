# ifc-style assignment instructions

Scope: style and presentation-layer associations to representation `EntityId`s.
Follow crate `../../AGENTS.md`; read `PLAN.md` only for assigned or remaining
assignment work.

## Owns

- `IfcStyledItem` and strict presentation-style select links
- presentation-layer assignment links
- deterministic association lookup and precedence
- explicit IFC2X3 presentation-style-assignment wrappers

## Does not own

- geometry-node imports
- renderer material creation
- texture loading

## Implementation map

- `styled_item.rs`: strict styled-item projections
- `layer.rs`: layer projections
- `resolution.rs`: deterministic direct-over-layer resolution

Keep views, resolution, validation, and neutral output in separate files.
