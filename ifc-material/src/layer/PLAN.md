# ifc-material layer plan

Status: IFC4 views implemented under `MAT-LAYER`. Last updated: 2026-08-20.
Follow `AGENTS.md`; claim one task and record blockers/decisions beneath it.

## Work queue

- [x] `LAYER-VIEW` - identity, material link, metadata, and authored thickness
  - Proof: `layers::reads_layers_offsets_sets_and_total_thickness` plus crate clippy.
- [x] `LAYER-SET` - ordered semantic membership and total authored thickness
  - Requires: `LAYER-VIEW`.
  - Proof: `layers::reads_layers_offsets_sets_and_total_thickness` and `strict_decoding::rejects_scalar_and_nested_schema_aggregates`.
- [x] `LAYER-USAGE` - authored usage direction, sense, offset, and extent
  - Requires: `LAYER-SET`.
  - Proof: facade `material_step`, `layers::reads_layer_set_usage_enums_and_dimensions`, and `errors::malformed_enums_cardinals_and_selects_are_explicit_errors`.
- [ ] `LAYER-CROSS` - shared fixture with geometry's material-usage projection
  - Requires: `LAYER-USAGE`, `INPUT-MAT`.
  - Proof: both projections join by EntityId without crate dependencies or duplicate slot parsing.

## Completion log

`LAYER-*` - `tests/layers.rs`, `errors.rs`, and facade STEP fixture; geometry
cross-projection remains separately owned by `LAYER-CROSS`.
