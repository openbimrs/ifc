# ifc-georef conversion plan

Status: planned under `GEOREF-MAP`. Last updated: 2026-08-19.
Follow `AGENTS.md`; claim one task and record material decisions beneath it.

## Work queue

- [x] `MAP-VIEW` - map conversion slots
  - Proof: tests/project_to_map.rs resolves_ifc4_map_conversion_into_a_metres_to_metres_transform.
- [x] `MAP-XFORM` - derive neutral transform
  - Proof: tests/project_to_map.rs converts_map_translation_and_scale_when_project_and_map_units_differ, defaults_each_missing_axis_component_independently.
- [x] `MAP-DEGEN` - reject degenerate axis/scale
  - Proof: tests/project_to_map.rs rejects_a_zero_axis_and_non_positive_scale, rejects_overflow_in_the_unit_normalized_scale/translation, refuses_map_conversion_scaled_until_unequal_factors_are_represented.
- [x] `MAP-EXAMPLE` - verify independent known coordinates
  - Proof: tests/project_to_map.rs resolves_a_committed_ifc_fixture_to_the_neutral_map_transform: synthetic_conic_offset_bounded.ifc, point (1,2,3) -> (2,4,3.01), target CRS EPSG:25832.

## Completion log

Append `TASK-ID - proof - material decision`; no long logs.
