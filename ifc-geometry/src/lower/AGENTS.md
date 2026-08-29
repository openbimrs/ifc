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
- exact curve/surface/profile/solid nodes (`curve.rs`, `csg.rs`)
- tessellated face sets as meshes (`tessellated`), never as topology
- mapped instances and boolean trees
- CSG solids, CSG primitives, and swept-disk solids (`csg.rs`)
- half spaces as boolean cutting tools (`halfspace.rs`)
- source provenance side table

Tessellated face sets are the one family that lowers to `axiolid-mesh`
rather than exact geometry. A face set is already a discretisation: it carries
no adjacency and no exactness claim, so building a `BRep` from it means
inferring shared edges by comparing floats. Authored n-gons and voids are kept
verbatim -- triangulation needs a fill rule and a tolerance, which are the
kernel's to choose. `CoordIndex` is 1-based and `PnIndex` (set-level and
per-face) is an extra addressing hop; both mistakes yield a mesh that renders
and is wrong, so both are mutation-covered.

Axes, normals, and orientation fields are finite non-zero unit-direction
candidates and are normalized exactly once at the IFC boundary. Displacements,
derivatives, scales, and other magnitude-bearing vectors preserve magnitude;
never normalize them merely because both use three scalar components.

A half space is INFINITE and is valid only as a boolean operand. Its
`AgreementFlag` is inverted relative to the neutral `HalfSpace.agreement`: IFC
`.T.` means the side the base surface normal points away from, the kernel's
`true` means the normal side. Getting it backwards cuts the wrong half and
produces a result that still evaluates and still looks like geometry.

A trim parameter belongs to the BASIS curve's parameterisation: a length on a
line, a plane angle on a conic. `lower::curve` selects the unit conversion from
the basis type, and `lower::csg` does the same for a swept disk's
`StartParam`/`EndParam` using the directrix type. A single length factor for
both is the defect this split exists to prevent.

A CSG primitive is local by kernel contract. Its `Position` is carried on an
`Instance` node, never folded into the primitive's extents.

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
