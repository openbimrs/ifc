# ifc-geometry input instructions

Scope: Geometry-affecting borrowed views from IFC resources outside the three geometry resources.

Follow the crate `../../AGENTS.md`.

## Owns

- ProfileResource shape references, profile-local placement, and void boundaries
- RepresentationResource Body/Axis/FootPrint selection, context, and precision
- MaterialResource profile references, cardinal/reference extent, layer usage direction/sense/offset, and taper geometry associations
- ProductExtension product shape/local-placement links
- ProfileResource family parameters (`profile/`): the ONE reader of
  `IfcProfileDef` slots, in SI units. `lower::profile` maps its description
  onto neutral profiles and reads only boundary curves itself; a second
  reader of the same slots would let a rule check and the kernel disagree.
- Body description (`body/`): representation kind and swept-solid
  parameters per item, in world coordinates

Body description decisions: a body is reported one entry per resolved item
(mapped items expanded, authored order), never merged or truncated; any item
that cannot be described fails the whole call. A mapping that scales or
mirrors a swept solid is refused, because its profile parameters would not be
the authored ones; kinds under such a mapping are still reported. Frames
compose exactly as lowering composes them (context, placement, mapping,
`Position`), and the crate's body-description test suite ties `BodyKind`
to the dispatcher's family list.

## Does not own

- material identity/style/quantity semantics
- domain-crate dependencies
- lowering or neutral geometry construction

## Growth map

`representation.rs`, `material_usage.rs`, `product.rs`, `profile/`, `body/`. These source owners already compile as private scaffold modules. Replace a module's planned-owner marker with its first real contract and tests; do not add parallel placeholders.

B-rep topology slot decoding is deliberately **not** owned here: `resource::topology` with `solid::brep` own it. Adding an `input` module for it would create a competing reader of the same slots.

Every source entity error cites EntityId/type/slot or rule. Add invalid, cycle,
and unsupported cases, not only happy paths.
