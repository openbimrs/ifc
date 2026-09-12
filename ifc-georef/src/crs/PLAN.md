# ifc-georef crs plan

Status: planned under `GEOREF-CRS`. Last updated: 2026-08-19.
Follow `AGENTS.md`; claim one task and record material decisions beneath it.

## Work queue

- [x] `CRS-VIEW` - projected CRS fields
  - Proof: tests/project_to_map.rs asserts target_crs.entity and target_crs.name (EPSG:25832) on resolved operations, including from a committed fixture.
- [x] `CRS-UNIT` - explicit map units
  - Proof: tests/project_to_map.rs asserts map_unit.metres_per_unit for both metres (1.0) and millimetres (0.001) in converts_map_translation_and_scale_when_project_and_map_units_differ.
- [ ] `CRS-DIAG` - missing/contradictory metadata
  - Proof: focused valid/invalid/edge fixture tests plus crate clippy.
  - Audited 2026-09-12: crs/unit.rs raises MissingEntity, MissingAttribute, WrongType, InvalidUnit and UnitCycle, but no test exercises any of them; only DegenerateAxis, InvalidAttribute, InvalidScale and UnsupportedOperation are covered. Still open.

## Completion log

Append `TASK-ID - proof - material decision`; no long logs.
