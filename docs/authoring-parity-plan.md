# Authoring parity plan

Status: active. Last updated: 2026-09-18.

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
| 1 | unit | done: 7 helpers | `0b616d4` |
| 2 | owner history | done: 5 helpers, in the generic layer | `c8912c1` |
| 3 | material | done: 12 helpers | `abbd19c` |
| 4 | cost | done: 7 helpers, in `mutation/` not `authoring.rs` | `96b9d0b` |
| 5 | structural | done: 8 helpers | `b3d67d2` |
| 6 | sequence | done: 9 helpers (3 task/sequence, 6 programme) | `16fd64c` |
| 7 | geometry authoring | done: 11 helpers, inside `ifc-geometry` (ADR 0011) | `98fe2a9` |
| 8 | alignment | done: 8 helpers | `f87aa87` |

Breadth goal met: all eight domains author. 87 public authoring functions,
counted 2026-09-18 across `*authoring*`, `ifc-cost/src/mutation/` and
`ifc-author/src/`.

Breadth is not depth. The baseline comparison (78 vs IfcOpenShell 384) was
a count of surface, and per-domain depth still varies widely -- sequence has
3 helpers against IfcOpenShell's ~40. Closing that is volume work, tracked
separately from this plan.

Every entity type `ifc-schedule` can read, it can now write: the four
remaining ones (`IfcEvent`, `IfcEventTime`, `IfcLagTime`,
`IfcRecurrencePattern`) landed with slot order verified against the
bundled EXPRESS schema rather than recalled. That probe found three
facts the readers had not exposed: `IfcLagTime.LagValue` and
`DurationType` are REQUIRED, `IfcEvent` carries a
`UserDefinedEventTriggerType` at slot 9, and the recurrence component
sets are `SET [1:?]`, so an empty aggregate is invalid where omission
is legal.

Depth is measured per domain against what the crate can already read.
`ifc-schedule` had 71 reader functions against 3 authoring helpers, and
ten entity types were readable but not authorable, including
`IfcWorkSchedule` itself. Six helpers closed the ones a programme needs:
work plans and schedules, calendars, working and exception periods, task
assignment and task nesting. `IfcEvent`, `IfcEventTime`, `IfcLagTime` and
`IfcRecurrencePattern` remain readable only.

## Lowering, which phase 8 depended on

Phase 8 was recorded as "blocked on alignment -> geometry lowering". That
lowering now exists:

- `lower_gradient_curve` / `gradient_curve3` compose a horizontal layout and
  a vertical profile into an exact `Curve3::Elevated` (`d8562f3`).
- `derive_placement_transform` resolves an `IfcLinearPlacement` that states
  only an `IfcPointByDistanceExpression`, by evaluating the basis curve
  through an injected `CurveEvaluator` (`bfb8a5e`).

Remaining refusals in this area, both deliberate:

- `VIENNESEBEND` now lowers exactly when the file states the cant layout
  and `GravityCenterLineHeight`; both absences are refused by name rather
  than defaulted, since a missing height is not zero.
- `Ellipse` / `BSpline` basis curves, refused at the distance API upstream:
  neither has a closed-form arc length. Polyline support is implemented here
  and waits on an `axiolid-evaluate` release (see `docs/local-kernel.md`).

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

Alignment (phase 8) has since landed too, and the lowering it waited on
exists -- see the section above. Deriving plans from solids stays upstream.
