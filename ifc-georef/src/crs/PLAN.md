# ifc-georef crs plan

Status: planned under `GEOREF-CRS`. Last updated: 2026-08-19.
Follow `AGENTS.md`; claim one task and record material decisions beneath it.

## Work queue

- [x] `CRS-VIEW` - projected CRS fields
  - Proof: tests/project_to_map.rs asserts target_crs.entity and target_crs.name (EPSG:25832) on resolved operations, including from a committed fixture.
- [x] `CRS-UNIT` - explicit map units
  - Proof: tests/project_to_map.rs asserts map_unit.metres_per_unit for both metres (1.0) and millimetres (0.001) in converts_map_translation_and_scale_when_project_and_map_units_differ.
- [x] `CRS-DIAG` - missing/contradictory metadata
  - Proof: tests/unit_diagnostics.rs (7 tests) covers MissingEntity, WrongType, InvalidUnit (unknown SI prefix), MissingAttribute and UnitCycle, plus a well-formed FOOT resolving to 0.3048 m. Both cycle guards are mutation-pinned independently.

## Completion log

Append `TASK-ID - proof - material decision`; no long logs.
