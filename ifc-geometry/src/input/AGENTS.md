# ifc-geometry input instructions

Scope: Geometry-affecting borrowed views from IFC resources outside the three geometry resources.

Follow the crate `../../AGENTS.md`. Read this directory's `PLAN.md` only for assigned
work under parent task(s) `GEOM-INPUT`. Record progress there.

## Owns

- ProfileResource shape references, profile-local placement, and void boundaries
- RepresentationResource Body/Axis/FootPrint selection, context, and precision
- MaterialResource profile references, cardinal/reference extent, layer usage direction/sense/offset, and taper geometry associations
- ProductExtension product shape/local-placement links
- TopologyResource topology consumed by B-rep lowering

## Does not own

- material identity/style/quantity semantics
- domain-crate dependencies
- lowering or neutral geometry construction

## Growth map

`representation.rs`, `material_usage.rs`, `product.rs`. These source owners already compile as private scaffold modules. Replace a module's planned-owner marker with its first real contract and tests; do not add parallel placeholders.

Profile and B-rep topology slot decoding are deliberately **not** owned here: `lower::profile` owns profile families, and `resource::topology` with `solid::brep` own B-rep topology. Adding an `input` module for either would create a competing reader of the same slots.

Every source entity error cites EntityId/type/slot or rule. Add invalid, cycle,
and unsupported cases, not only happy paths.
