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

## Progress

| Phase | Domain | State | Commit |
| --- | --- | --- | --- |
| 1 | unit | done: 8 helpers, 6 tests, 2 mutations caught | `0b616d4` |
| 2 | owner history | done: 5 helpers, 4 tests, 3 mutations caught | `c8912c1` |
| 3 | material | next: close the 16 -> 25 gap (profile/constituent sets) | |
| 4 | cost | pending: 6 -> 20 | |
| 5 | structural | pending: 2 -> 23, 14 stub files to fill | |
| 6 | sequence | pending: 1 -> 40, ifc-schedule is 58% stubs | |
| 7 | geometry authoring | pending: 0 -> 30, needs a representation-builder story | |
| 8 | alignment | pending: 0 -> 60, blocked on alignment->geometry lowering | |

## Conventions established in phases 1-2

- Authoring helpers stage onto `ifc_model::Transaction`; they never commit.
- A `*Draft` struct carries authored fields; required schema attributes are
  plain fields, optional ones are `Option`.
- Slot order is verified against the bundled schema with a throwaway probe
  before the helper is written, never from memory. Delete the probe after.
- Refusals target mistakes that PARSE and VALIDATE but corrupt meaning --
  a wrong SI prefix, a record modified before it existed. Schema-level
  errors are `ifc-validate`'s job, not the author's.
- Every guard is mutation-tested before it is trusted.

## Phase 7 outcome (geometry authoring)

Decided by spike, recorded in ADR 0011: authoring lives **inside**
`ifc-geometry`, not in a separate crate. 183 of the 201 slot constants an
authoring API needs were unreachable externally, and `RepresentationPurpose`
is invisible without linking the kernel a separate crate exists to avoid.

Vendor neutrality comes from the kernel being **absent** from the API, not
abstracted over: authoring takes `f64`/`[f64; 3]`/index buffers and runs in
the `--no-default-features` column, enforced by `kernel_free_build.rs`.

Remaining: alignment (phase 8), still blocked on alignment -> geometry
lowering, which does not exist. Deriving plans from solids stays upstream.
