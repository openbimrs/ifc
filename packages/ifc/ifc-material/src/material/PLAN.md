# ifc-material material plan

Status: core IFC4 views implemented under `MAT-BASE`. Last updated: 2026-08-20.
Follow `AGENTS.md`; claim one task and record blockers/decisions beneath it.

## Work queue

- [x] `MATDEF-VIEW` - typed material identity accessors
  - Proof: `definitions::reads_material_identity_constituents_and_sets` plus crate clippy.
- [x] `MATDEF-REL` - material property/classification/resource relationships
  - Proof: `definitions::reads_material_properties_relationships_and_lists` and facade template tests.
- [x] `MATDEF-TEST` - missing/duplicate/unknown references
  - Proof: `strict_decoding::assignment_resolution_rejects_missing_objects_and_malformed_relations`.

## Completion log

`MATDEF-*` - `tests/definitions.rs`, `errors.rs`, and template facade tests;
inverse representation projection remains outside this MaterialResource slice.
