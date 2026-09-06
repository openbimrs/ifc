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
- [x] `RULE-ENFORCE` - enforce the inventoried geometry-resource WHERE rules
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
  - Progress (2026-09-06): all 95 rows enforced, up from 16 at the start of
    the batch. Rule modules: `dimension.rs` (the `IfcCurveDim` derivation),
    `curve.rs`, `scalar.rs`, `cardinality.rs`, `typing.rs`, `surface.rs`,
    `bspline.rs`, and `express.rs` -- the last holding transcriptions of the
    schema's own normative FUNCTIONs, unit-tested against the specification.
  - Readers live under their declared owners, not inside `rules/`:
    - `surface::basis` -- `IfcGetBasisSurface` and `IfcAssociatedSurface`,
      feeding `SameSurface` (composite and seam) and `DistinctSurfaces`.
    - `solid::brep::non_advanced_faces` -- shell face traversal, feeding
      `HasAdvancedFaces` and `VoidsHaveAdvancedFaces`.
  - Two places where the schema text and its stated intent diverge, both
    recorded at the code that decides them:
    - `IfcGetBasisSurface` loops `Segments[1]` rather than `Segments[i]`, so
      read literally the composite intersection is a no-op and `SameSurface`
      can never fail. The documented intent is implemented instead; the
      quoted EXPRESS sits in the module header.
    - `DirectrixBounded` counts a set intersection and demands exactly one
      match. No IFC4 entity is both a conic and a bounded curve, so only the
      zero case is reachable, and only that case is reported rather than
      carrying an untestable branch for an impossible state.
  - `FunctionStatus::Implemented` distinguishes an executed function from a
    `Scaffolded` placeholder. `declaration_manifest.rs` fails when a row
    claims `Implemented` without its owner module naming the function, which
    is what caught eight rows that were understating the crate.

## Completion log

Append `TASK-ID - proof - material decision`; keep long logs out of this file.
