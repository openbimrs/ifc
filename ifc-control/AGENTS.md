# ifc-control instructions

Purpose: bounded IFC control authoring for permits, project orders, action requests, and performance history.

Follow `../AGENTS.md`.

## Implemented boundary

- `IfcPermit`, `IfcProjectOrder`, `IfcActionRequest`, `IfcPerformanceHistory`;
- transaction-staged authoring with per-entity predefined-type enums.

A control is an authored fact. Issuing, granting, approving, scheduling, or
enforcing one is the concern of the system that owns the process, not this
crate. Controls attach to the work they govern through `IfcRelAssignsToControl`.
`ifc-cost` and `ifc-schedule` already write that relationship for their own
controls; whether this crate does the same for its four is #99, and until
then it does not create them.

`IfcCostItem`, `IfcCostSchedule`, `IfcWorkCalendar` and `IfcWorkControl` are
`IfcControl` subtypes too, but they belong to `ifc-cost` and `ifc-schedule`:
crates split by domain, not by supertype. Do not move them here.

`IfcWorkOrder` does not exist in IFC: a work order is an `IfcProjectOrder`
whose `PredefinedType` is `WORKORDER`. Do not add an entity for it.

## Boundary

Allowed production dependencies: `ifc-model` and `ifc-schema` only; no
geometry crate and no codec. Authoring stages on a caller-owned
transaction; this crate never commits or performs external I/O.

## Module ownership

- `src/authoring.rs`: `ControlKind`, typed drafts, and transaction staging
- `src/error.rs`: typed authoring refusals
- `tests/control.rs`: public behavior, refusals, and schema agreement

## Invariants

- Arity comes from the schema tables, never a hardcoded slot count.
- Type names are stored upper-case; `Entity::new` does not normalise, and
  a mixed-case record is invisible to `of_type` lookups.
- `USERDEFINED` requires `ObjectType`; the token asserts a name given
  elsewhere, and `IfcObject` puts it there.
- `IfcPerformanceHistory` carries a required `LifeCyclePhase` at slot 6 and
  declares neither `Status` nor `LongDescription`. An attribute the entity
  does not declare is refused, never dropped.
- Rejected drafts leave transaction length unchanged.

## Verification

Run focused tests, strict all-target Clippy/rustdoc, the control mutation
sweep, then the full repository gate.
