# ifc-geometry resource instructions

Scope: Zero-copy views for IfcGeometryResource values and helper functions.

Follow the crate `../../AGENTS.md`.

## Owns

- points/directions/axes/placements/operators
- representation maps and geometric helper functions
- absolute STEP slot constants and accessor errors
- `resolve.rs`: the one place a raw reference becomes a type-checked view.
  Curve/surface/solid views keep their `*_ref` getter and add a resolving
  accessor beside it (`Line::point(&model)`) that calls `resolve`, so dangling
  (`MissingEntity`, names the referrer) and wrong-type (`WrongEntityType`,
  names the target) errors are identical everywhere. An attribute typed by an
  abstract supertype (`IfcSurface`, `IfcBoundedCurve`) stays a reference: the
  dispatch belongs to `lower`. `IfcProfileDef` has no view here either;
  `input::profile` owns profile reading.

## Does not own

- recursive model traversal
- unit conversion or transform composition
- kernel/graph types

## Growth map

`point.rs`, `direction.rs`, `axes.rs`, `placement.rs`, `operator.rs`, `mapped.rs`, `functions.rs`, `resolve.rs`. These source owners already compile as private scaffold modules. Replace a module's planned-owner marker with its first real contract and tests; do not add parallel placeholders.

Every source entity error cites EntityId/type/slot or rule. Add invalid, cycle,
and unsupported cases, not only happy paths.
