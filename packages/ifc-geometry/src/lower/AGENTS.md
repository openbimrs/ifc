# ifc-geometry lower instructions

Scope: Total translation from validated IFC views to an exact format-neutral GeometryGraph.

Follow the crate `../../AGENTS.md`. Read this directory's `PLAN.md` only for assigned
work under parent task(s) `GEOM-CONTRACT, GEOM-SESSION, GEOM-CTX, GEOM-PLACE,
GEOM-PROFILE, GEOM-CURVE, GEOM-SURFACE, GEOM-BREP, GEOM-SOLID, GEOM-MAP`.
Record progress there.

## Owns

- dispatch and recursion budgets
- one shared graph builder, memo table, active recursion stack, and provenance map
- unit/frame/context composition
- exact curve/surface/profile/solid nodes
- mapped instances and boolean trees
- source provenance side table

Axes, normals, and orientation fields are finite non-zero unit-direction
candidates and are normalized exactly once at the IFC boundary. Displacements,
derivatives, scales, and other magnitude-bearing vectors preserve magnitude;
never normalize them merely because both use three scalar components.

## Does not own

- kernel execution or backend selection
- implicit tessellation/flattening
- semantic material/style/quantity handling

## Growth map

`session.rs`, `dispatch.rs`, `context.rs`, `placement.rs`, `profile.rs`,
`curve.rs`, `surface.rs`, `solid.rs`, `brep.rs`, `tessellated.rs`, `mapped.rs`,
`boolean.rs`, `provenance.rs`. These source owners already compile as private scaffold modules. Replace a module's planned-owner marker with its first real contract and tests; do not add parallel placeholders.

Every source entity error cites EntityId/type/slot or rule. Add invalid, cycle,
and unsupported cases, not only happy paths.
