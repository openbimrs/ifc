# ifc-alignment horizontal plan

Status: planned under `ALIGN-H`. Last updated: 2026-08-19.
Follow `AGENTS.md`; claim one task and record material decisions beneath it.

## Work queue

- [x] `HOR-LAYOUT` - ordered segment traversal
  - Proof: tests/layout_and_placement.rs horizontal_layout_assembles_a_continuous_composite_curve asserts ordered sources [101, 103] from a committed fixture.
- [ ] `HOR-SEG` - line/arc/transition views
  - Proof: focused valid/invalid/edge fixture tests plus crate clippy.
  - Audited 2026-09-12: line/arc/transition views are exercised only indirectly through lower_horizontal_layout; no test calls read_horizontal_segment directly and horizontal/segment.rs has no in-source tests. Still open.
- [x] `HOR-CONT` - position/tangent continuity tests
  - Proof: same test asserts per-segment Transition::Discontinuous then Transition::Continuous; a_clothoid_segment_inside_a_layout_is_a_typed_refusal_not_an_approximation covers the refusal path.

## Completion log

Append `TASK-ID - proof - material decision`; no long logs.
