# ifc-control instructions

Purpose: bounded IFC control authoring for permits, project orders, action requests, and performance history.

Follow `../../AGENTS.md`.

## Implemented boundary

- `IfcPermit`, `IfcProjectOrder`, `IfcActionRequest`, `IfcPerformanceHistory`;
- transaction-staged authoring with per-entity predefined-type enums;
- `IfcRelAssignsToControl` whose relating control is one of those four.

A control is an authored fact. Issuing, granting, approving, scheduling, or
enforcing one is the concern of the system that owns the process, not this
crate. Controls attach to the work they govern through `IfcRelAssignsToControl`,
and the settled rule is: **the crate owning the relating control writes the
assignment.** This crate writes it for its four controls and refuses any other
relating control (`ControlError::ForeignControl`); `ifc-cost` writes it for
cost schedules and `ifc-schedule` for work controls. `RelatedObjectsType` is
always left unset: optional in IFC4 (its WR1 passes when absent) and a
stripped BOOLEAN in IFC4X3.

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
- `src/assignment.rs`: staging `IfcRelAssignsToControl` for owned controls
- `src/error.rs`: typed authoring refusals
- `tests/control.rs`: public behavior, refusals, and schema agreement
- `tests/assignment.rs`: assignment staging, ownership and `RelatedObjects` refusals

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
- `RelatedObjects` is `SET [1:?]`: empty, duplicated, self-referencing
  (`NoSelfReference`) and non-`IfcObjectDefinition` members are refused.

## Verification

Run focused tests, strict all-target Clippy/rustdoc, the control mutation
sweep, then the full repository gate.
