# ifc-alignment vertical plan

Status: planned under `ALIGN-V`. Last updated: 2026-08-19.
Follow `AGENTS.md`; claim one task and record material decisions beneath it.

## Work queue

- [x] `VER-LAYOUT` - ordered profile traversal
  - Proof: tests/layout_curves.rs vertical traversal via constant_gradient_lowers_to_an_exact_neutral_segment; ordering and unit resolution asserted in tests/parameter_segments.rs.
- [ ] `VER-SEG` - gradient/arc/parabola views
  - Proof: focused valid/invalid/edge fixture tests plus crate clippy.
  - Audited 2026-09-12: parse_type maps CONSTANTGRADIENT, CIRCULARARC, PARABOLICARC and CLOTHOID, but only ConstantGradient is asserted (tests/parameter_segments.rs). No test reads an arc or parabola segment view. Still open.
- [ ] `VER-CONT` - station/elevation/slope continuity
  - Proof: focused valid/invalid/edge fixture tests plus crate clippy.
  - Audited 2026-09-12: no test asserts station/elevation/slope continuity across adjacent vertical segments; only a single-segment read is covered. Still open.

## Completion log

Append `TASK-ID - proof - material decision`; no long logs.
