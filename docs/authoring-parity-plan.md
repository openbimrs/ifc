# Authoring parity plan

Status: active. Last updated: 2026-09-14.

## Goal

Reach authoring-breadth parity with IfcOpenShell's Python API across eight
domains: unit, owner, material, cost, structural, sequence, geometry, alignment.

Baseline measured 2026-09-14: openbim 78 public authoring functions vs
IfcOpenShell 384 (counted as .py files under api/ excluding __init__.py).

## Ordering rationale

Dependency order, not size order:

1. **unit** - every numeric attribute is meaningless without unit context.
2. **owner** - IfcOwnerHistory is required on every IfcRoot subtype.
3. **material/cost/structural** - existing crates, extend in place.
4. **sequence** - IfcWorkSchedule/task graph; ifc-schedule is 58% stubs.
5. **geometry** - authoring representations (not lowering).
6. **alignment** - largest; depends on geometry authoring existing first.

## Architecture constraint

`ifc-model/tests/package_architecture.rs` forbids domain crates from depending
on siblings: allowed deps are GENERIC (ifc-model, ifc-schema) only.

Consequence: owner history is needed by every domain, so it CANNOT live in a
sibling domain crate. It belongs in the generic layer.

## House pattern (from ifc-material/src/authoring.rs)

- `XxxDraft<'a>` struct holds authored attribute values.
- `pub fn create_xxx(tx: &mut Transaction, model: &Model, draft) -> Result<EntityId>`.
- Validate BEFORE staging: scalar ranges, GUID format, reference types.
- `require_type` / `require_exists` resolve against staged edits first, then model.
- Only stage; `Transaction::commit` owns atomic application.
