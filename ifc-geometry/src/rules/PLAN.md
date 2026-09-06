# ifc-geometry rules plan

Status: active scaffold under parent task(s) `GEOM-CENSUS`.
Last updated: 2026-08-19

Follow `AGENTS.md`. Claim one local task, leave blockers/decisions beneath it,
and check it off only after the proof runs.

## Work queue

- [x] `RULE-REG` - inventory all relevant WHERE rules and support state
  - Done (2026-09-05): `data/ifc4-where-rules.tsv` lists all 95 geometry-
    resource rules across 56 entities, each `implemented` or `inventoried`.
    `tests/where_rule_inventory.rs` regenerates it from the bundled schema
    and fails on drift in either direction. Needs openbim-step >= 0.5.0,
    which captures `EntityDef::where_rules`.
- [x] `RULE-PLACE` - placement/direction dimensional rules
  - Audited (2026-09-05): Placement and direction dimensional rules are implemented and tested.
  - Proof: focused tests, crate clippy, and relevant declaration/corpus gate.
- [x] `RULE-SOLID` - swept/boolean/half-space rules
  - Audited (2026-09-05): Swept, boolean and half-space rules are implemented and tested.
  - Proof: focused tests, crate clippy, and relevant declaration/corpus gate.
- [x] `RULE-PROP` - pair every violation test with a conforming edge case
  - Audited (2026-09-05): Each violation test is paired with a conforming edge case.
  - Proof: focused tests, crate clippy, and relevant declaration/corpus gate.
- [x] `RULE-REPORT` - unsupported vs failed vs passed are distinct
  - Audited (2026-09-05): Unsupported, failed and passed are distinct variants and asserted as such.
  - Proof: focused tests, crate clippy, and relevant declaration/corpus gate.
- [ ] `RULE-ENFORCE` - enforce the inventoried geometry-resource WHERE rules
  - Goal: move rows from `inventoried` to `implemented` in
    `data/ifc4-where-rules.tsv`, each with a pass/fail case in
    `tests/where_rule_inventory.rs`. The coverage assertion in that test
    already fails when a row claims implementation without a case, so the
    inventory cannot drift ahead of the code.
  - The 78 open rows sort into five shapes. Order is by leverage: a shared
    helper closes a whole shape at once.
    - A. Dimensionality (18): `X.Dim = 2|3` and `SameDim` across a list.
      One `dim_of(entity)` resolver plus a list-agreement helper.
    - B. Positive scalars (8): `Scl > 0`, `Depth > 0`, `Magnitude >= 0`.
      Beware `>=` vs `>`: `IfcVector.MagGreaterOrEqualZero` admits zero.
    - C. Cardinality (11): `SIZEOF(a) = SIZEOF(b)` correspondence between
      parallel lists (knots/multiplicities, weights/control points,
      cross-sections/positions).
    - D. Type membership (17): `'IFC4.IFCX' IN TYPEOF(slot)`. Uses the
      existing `select::is_a` subtype table; watch exact-vs-subtype, the
      schema means exact type for some rows.
    - E. Schema functions (8): `IfcConstraintsParamBSpline`,
      `IfcTaperedSweptAreaProfiles`, `IfcCorrectLocalPlacement`,
      `IfcConsecutiveSegments`, `IfcCurveWeightsPositive`. These are
      normative EXPRESS functions, not one-line predicates.
  - Constraint: a rule marked `implemented` MUST fire on violating data and
    stay silent on conforming data. A rule whose check cannot fail (see the
    `IfcBooleanResult.SameDim` dead-branch finding) stays `inventoried` with
    the reason recorded next to the code.
  - Evidence: `tests/where_rule_inventory.rs` pass/fail cases per row, plus
    mutation probes on each new helper.
  - Progress (2026-09-06): 63 of 95 rows enforced, up from 16. New modules
    `dimension.rs` (the `IfcCurveDim` derivation), `curve.rs`, `scalar.rs`,
    `cardinality.rs`, `typing.rs`, `surface.rs`.
  - Remaining 32 rows and why each is still `inventoried`:
    - Normative EXPRESS functions (9): `IfcConstraintsParamBSpline`,
      `IfcTaperedSweptAreaProfiles`, `IfcCorrectLocalPlacement`,
      `IfcConsecutiveSegments`, `IfcCurveWeightsPositive`,
      `IfcSurfaceWeightsPositive`. Each is a multi-branch algorithm; they
      deserve their own transcription task with their own fixtures.
    - `IfcGetBasisSurface` dependants (4): `SameSurface`, `DistinctSurfaces`,
      `IsClosed`, `CompositeCurveOnSurface.SameSurface`. Need the derived
      basis-surface set, which walks p-curve to surface identity.
    - B-spline SURFACE lists (3): the surface analogue of the curve
      cardinality rules; needs the 2D control grid reader.
    - `Closed` on tessellated operands (2), `CurveContinuous` transition
      counting (1), `DirectrixBounded` (4), advanced-face membership (2),
      revolved-axis-in-XY (2), `ConsistentProfileTypes` (1),
      `ApplicableMappedRepr` (1), `UsenseCompatible` (1),
      `Trim1/2ValuesConsistent` (2): each needs a reader this crate does
      not yet have, or a select the rules layer cannot see.
    - `IfcBooleanResult.SameDim` (1): stays `inventoried` because its
      check cannot fail -- see the dead-branch note in `solid.rs`.

## Completion log

Append `TASK-ID - proof - material decision`; keep long logs out of this file.
