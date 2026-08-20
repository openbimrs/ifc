# ifc-material usage plan

Status: IFC4 resolution implemented under `MAT-ASSIGN`. Last updated: 2026-08-20.
Follow `AGENTS.md`; claim one task and record blockers/decisions beneath it.

## Work queue

- [x] `MATUSG-REL` - association views
  - Proof: `assignments::occurrence_assignment_overrides_type_and_type_is_fallback` plus crate clippy.
- [x] `MATUSG-RESOLVE` - occurrence/type precedence contract
  - Proof: `assignments.rs` pins occurrence precedence and type fallback.
- [x] `MATUSG-BUDGET` - one-hop type traversal and ambiguity tests
  - Proof: `errors::duplicate_occurrence_assignments_are_not_guessed` and `strict_decoding::duplicate_type_relations_are_ambiguous_even_for_the_same_type`.

## Completion log

`MATUSG-*` - `tests/assignments.rs` and `errors.rs`; IFC type fallback is one
bounded relation hop and direct occurrence assignment wins.
