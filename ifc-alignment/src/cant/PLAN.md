# ifc-alignment cant plan

Status: planned under `ALIGN-CANT`. Last updated: 2026-08-19.
Follow `AGENTS.md`; claim one task and record material decisions beneath it.

## Work queue

- [x] `CANT-LAYOUT` - ordered traversal
  - Proof: tests cant_layout_resolves_the_full_profile_and_queries_by_distance; inconsistent_segment_semantics_are_typed_errors.
- [x] `CANT-SEG` - segment parameter views
  - Proof: cant::evaluate::tests cover all seven IfcAlignmentCantSegmentTypeEnum members (constant, linear, bloss, cosine, sine, helmert, vienna) plus resolves_cant_parameters_and_signed_offsets.
- [x] `CANT-CONT` - unit/continuity/side tests
  - Proof: cant::evaluate::tests helmert_curve_is_continuous_across_its_own_midpoint_seam, sine_curve_starts_and_ends_exactly_on_the_endpoints, bloss_curve_is_symmetric_boundary_exact, vienna_bend_reproduces_the_endpoints_exactly, transitions_refuse_a_missing_end_value.

## Completion log

Append `TASK-ID - proof - material decision`; no long logs.
