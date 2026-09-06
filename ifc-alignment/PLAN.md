# ifc-alignment implementation plan

Status: horizontal/vertical/cant segment parameters implemented; exact neutral line/circular-horizontal and constant-gradient-vertical output implemented; IFC4X3 schema pinning (ALIGN-VERS), continuity-aware horizontal composite curve assembly (ALIGN-CURVE), full closed-form cant segment/layout evaluation (ALIGN-CANT), and linear placement + station-equation resolution (ALIGN-PLACE) are implemented. Transition spirals reconstructible from endpoint radii (CLOTHOID, BLOSSCURVE, COSINECURVE) lower exactly as intrinsic curves carrying `CurvatureLaw` -- lossless, no quadrature or sampling. HELMERTCURVE, SINECURVE, and VIENNESEBEND remain a typed refusal: they need terms the segment does not carry. Runs still split after a spiral because continuity out of one is not closed form.
Last updated: 2026-09-06

This is task state, not ambient context. Follow `AGENTS.md`; claim one task ID,
record blockers/decisions under it, and check it off only with evidence.

## Established boundary

Interpret IFC4x3 alignment intent into exact neutral curves/frames without meshing or backend selection.

## Planned file map

These paths are compiled private scaffold modules. Implement inside the named
owner and expose a public symbol only through an intentional parent re-export.

- `src/alignment/root.rs`: IfcAlignment hierarchy
- `src/view.rs`: implemented -- pins the IFC4X3 schema (ALIGN-VERS) and exposes
  bounded `IfcRelNests` traversal (`nested_children`/`segment_chain`) shared by
  every layout module
- `src/horizontal/layout.rs`: segment order and continuity
- `src/horizontal/segment.rs`: line/arc/transition parameters
- `src/vertical/layout.rs`: profile order
- `src/vertical/segment.rs`: gradients/arcs/parabolas
- `src/cant/layout.rs`: implemented -- `CantLayout` resolves, orders, and
  continuity-checks a full `IfcAlignmentCant` profile (ALIGN-CANT)
- `src/cant/segment.rs`: cant transitions
- `src/cant/evaluate.rs`: implemented -- closed-form `D(ξ)` for all seven
  `IfcAlignmentCantSegmentTypeEnum` members (every cant type has an explicit,
  exact base formula in the spec -- no integration needed)
- `src/curve/assemble.rs`: implemented -- exact neutral composite curve
  (`lower_horizontal_layout`, ALIGN-CURVE) plus the single-segment lowerers
- `src/placement/linear.rs`: implemented -- `IfcLinearPlacement` and
  `IfcPointByDistanceExpression` resolution (ALIGN-PLACE)
- `src/placement/distance.rs`: point-by-distance expressions
- `src/referent/station.rs`: implemented -- `Pset_Stationing` station-equation
  resolution (ALIGN-PLACE)

- `src/cant/transition.rs`: compiled private scaffold; implementation owned by `src/cant/PLAN.md`
- `src/curve/provenance.rs`: compiled private scaffold; implementation owned by `src/curve/PLAN.md`
- `src/curve/transition.rs`: compiled private scaffold; implementation owned by `src/curve/PLAN.md`
- `src/horizontal/transition.rs`: compiled private scaffold; implementation owned by `src/horizontal/PLAN.md`
- `src/placement/station.rs`: compiled private scaffold; implementation owned by `src/placement/PLAN.md`
- `src/vertical/transition.rs`: compiled private scaffold; implementation owned by `src/vertical/PLAN.md`

## Work queue

- [x] `ALIGN-VERS` - pin the authoritative IFC4x3 profile and declaration inventory
  - `AlignmentView::for_model` accepts only `IFC4X3`/`IFC4X3_ADD2`; IFC2X3 and
    IFC4 are refused with `AlignmentError::UnsupportedSchema` since
    `IfcAlignment*` entities do not exist in either schema at all (unlike
    `ifc-resource`/`ifc-structural`, there is no cross-version dispatch table
    here -- exactly one profile is authoritative).
  - Evidence: `cargo test -p ifc-alignment view::` (6 tests: missing/ambiguous/
    unrecognized/refused/accepted schema tokens).
- [ ] `ALIGN-H` - implement exact horizontal segment views/lowering
  - Progress: parameters resolve with units; line and circular arc lower exactly;
    CLOTHOID, BLOSSCURVE and COSINECURVE lower exactly as `Curve2::Intrinsic`
    (kernel v0.12.0's natural-equation curve): their curvature law is
    elementary in arc length and reconstructible from the endpoint radii, so
    storing it is lossless. HELMERTCURVE (piecewise-quadratic), SINECURVE and
    VIENNESEBEND still fail typed -- their laws need terms the alignment
    segment does not carry. CUBIC (an exact literal polynomial in IFC's own
    definition) remains open.
  - Evidence: focused unit/property/fixture tests, isolated build, and crate clippy.
- [ ] `ALIGN-V` - implement exact vertical profile views/lowering
  - Progress: parameters resolve with units; constant gradient lowers exactly;
    CIRCULARARC/PARABOLICARC (also exact) and CLOTHOID (transcendental, same
    boundary as horizontal) remain open.
  - Evidence: focused unit/property/fixture tests, isolated build, and crate clippy.
- [x] `ALIGN-CANT` - implement exact cant views/lowering
  - All seven `IfcAlignmentCantSegmentTypeEnum` members (BLOSSCURVE,
    CONSTANTCANT, COSINECURVE, HELMERTCURVE, LINEARTRANSITION, SINECURVE,
    VIENNESEBEND) have an explicit closed-form `D(ξ)` base formula in the
    IFC4.3 spec itself -- unlike horizontal/vertical transitions, cant is the
    elevation value directly, not an integrated Cartesian position, so every
    type is exactly representable with no new geometry primitive.
    `CantLayout::resolve` orders segments, checks C0 continuity across the
    profile, and exposes a single station-domain query.
  - Evidence: `cargo test -p ifc-alignment cant::evaluate::` (13 tests, all
    seven formulas checked against the published base-formula values at
    interior points) plus
    `cant_layout_resolves_the_full_profile_and_queries_by_distance` in
    `tests/layout_and_placement.rs`.
- [x] `ALIGN-CURVE` - assemble continuity-aware neutral curves
  - `lower_horizontal_layout` walks an `IfcAlignmentHorizontal`'s nested
    segment chain via `AlignmentView::segment_chain`, lowers each exactly
    representable segment, and assembles a single `CurveRelation::Composite`
    with an observed (not assumed) `Transition` between consecutive segments --
    exact-equality endpoint matching, refusing rather than silently patching a
    gap.
  - Evidence: `horizontal_layout_assembles_a_continuous_composite_curve` and
    `a_clothoid_segment_inside_a_layout_is_a_typed_refusal_not_an_approximation`
    in `tests/layout_and_placement.rs`.
- [x] `ALIGN-PLACE` - implement linear placement and station equations
  - `resolve_linear_placement`/`resolve_point_by_distance` resolve
    `IfcLinearPlacement` -> `IfcAxis2PlacementLinear` -> the mandatory
    `IfcPointByDistanceExpression` (IFC4X3 WR1 forbids any other `Location`
    type on a linear axis placement). `station_equations` resolves every
    `IfcReferent` carrying a `Pset_Stationing` property set into its
    distance-along/station mapping, keeping `IncomingStation` distinct from
    `Station` per the spec's own station-equation semantics.
  - Evidence: `linear_placement_resolves_the_point_by_distance_expression` and
    `station_equations_resolve_from_pset_stationing_and_the_linear_placement`
    in `tests/layout_and_placement.rs`.
- [x] `ALIGN-PARTIAL` - lower spiral-bearing layouts per segment, not per layout
  - `lower_horizontal_layout_partial` keeps maximal runs of consecutive
    exactly-lowered segments and reports each refused segment with its entity
    id and authored `PredefinedType`. A run ends at every refusal, because
    continuity across a segment this crate did not lower is not a fact it can
    assert. The all-or-nothing `lower_horizontal_layout` is unchanged, so
    existing callers keep their exact contract. This closes the capability
    cliff where a single CLOTHOID failed an entire production alignment; it
    adds no approximation, only reporting.
  - Evidence: `tests/partial_lowering.rs` (4 tests) against the new
    `synthetic_alignment_spiral.ifc` line->clothoid->arc->clothoid->line
    fixture; mutation-checked by removing the run-flush and by silently
    lowering spirals as lines, both of which fail the suite.

- [x] `ALIGN-SPIRAL` - lower transition spirals exactly as intrinsic curves
  - `Curve2::Intrinsic` (axiolid-curve v0.12.0) stores a curvature law
    anchored to a start frame. CLOTHOID, BLOSSCURVE and COSINECURVE have
    elementary curvature laws reconstructible from the segment's endpoint
    radii, so this is a lossless representation, not an approximation: the
    crate still performs no integration anywhere. HELMERTCURVE, SINECURVE and
    VIENNESEBEND stay refused -- their laws need terms the segment does not
    carry, and forcing them into a nearby law would be a silent lie.
  - Continuity out of a spiral is still not assertable (Fresnel end point),
    so `lower_horizontal_layout_partial` ends a run after one and the strict
    entry point refuses such a layout outright.
  - Evidence: `tests/spiral_law.rs` pins each family at k(0), k(L/2), k(L)
    and total turning against the published base formulas; integrating the
    stored clothoid law reproduces the fixture's independently authored arc
    start to 1e-9. Mutation-checked with five mutants (clothoid sharpness,
    Bloss cubic sign, Bloss quadratic coefficient, cosine frequency, cosine
    phase) -- all caught.
- [ ] `ALIGN-CENSUS` - fixture/declaration coverage with explicit unsupported cases
  - Evidence: focused unit/property/fixture tests, isolated build, and crate clippy.

## Completion log

Append concise entries as `TASK-ID - proof command/result - material decision`.

- `ALIGN-H/V/C` parameter/exact slice - `cargo +1.88.0 test -p ifc-alignment`
  and the workspace gate pass against unit and committed IFC fixture tests;
  unsupported transitions are not approximated.
- `ALIGN-VERS/CURVE/CANT/PLACE` - `cargo +1.88.0 test -p ifc-alignment
  --all-targets` (36 tests, 0 failures), `cargo +1.88.0 clippy -p ifc-alignment
  --all-targets -- -D warnings` (clean), `cargo +1.88.0 test --workspace
  --all-targets` (0 regressions, one pre-existing corpus fixture-count
  assertion updated 32->33 for the new committed fixture), full
  `scripts/gate.sh` in an isolated `CARGO_TARGET_DIR`. Material decision:
  clothoid-family transition curves (CLOTHOID, HELMERTCURVE, BLOSSCURVE,
  COSINECURVE, SINECURVE, VIENNESEBEND) on horizontal/vertical position stay a
  typed refusal -- their Cartesian position is a genuine Fresnel-type integral
  with no closed form, so approximating it would violate the "never silently
  substitute geometry" invariant, and adding a transcendental curve primitive
  belongs upstream in `axiolid-curve`, not in this IFC bridge. Cant, by
  contrast, is fully closed for every defined type (the spec states `D(ξ)`
  directly, no integration), so `ALIGN-CANT` is complete.

- `ALIGN-PARTIAL` - `cargo +1.88.0 test -p ifc-alignment --all-targets`
  (40 tests, up from 36) plus the full workspace suite (168 test-result
  lines from `cargo test --workspace`, 231 across the whole `scripts/gate.sh`
  run, zero failures in both). `lower_horizontal_layout_partial` splits a layout
  into maximal exactly-lowerable runs and reports refused segments by entity
  id and authored type; `lower_horizontal_layout` keeps its all-or-nothing
  contract byte-for-byte, proven by asserting the partial path reproduces its
  graph structurally on a spiral-free layout. Mutation-checked: removing the
  run-flush on refusal, and silently lowering spirals as lines, each fail the
  suite -- so the gate can actually fail. Kernel repinned to axiolid v0.12.0
  (`Curve2::Intrinsic`) as a no-op verified against the pre-change baseline.
  New fixture `synthetic_alignment_spiral.ifc` (line->clothoid->arc->
  clothoid->line, the canonical production shape) validates clean in the
  ifc-validate corpus.

- `ALIGN-SPIRAL` - `cargo +1.88.0 test -p ifc-alignment --all-targets`
  (47 tests, up from 40) and the full workspace suite (169 test-result lines,
  zero failures); clippy `-D warnings`, fmt and rustdoc clean. Writing the
  family tests found a real bug: the cosine law had been written in cosine
  form while the kernel's `Sinusoid` is sine form, giving k(0) != 0. Fixed by
  folding via sin(x - pi/2) = -cos(x). Before those tests existed, mutating
  the Bloss and cosine coefficients failed nothing -- the fixture only
  exercises clothoids -- so the gap was real and is now closed.

Do not paste long logs or move standing invariants out of `AGENTS.md`.
