# ifc-model index plan

Status: planned under `MODEL-INV`. Last updated: 2026-08-19.
Follow `AGENTS.md`; claim one task and record blockers/decisions beneath it.

## Work queue

- [x] `INDEX-TYPE` - preserve current type lookup contract
  - Proof: tests/type_index_consistency.rs (3 tests: reinsertion does not duplicate, replacement clears the old entry, histogram agrees after replacement).
- [x] `INDEX-REV` - target-to-source/slot reverse references
  - Proof: tests/reverse_index.rs (5 tests: unreferenced target, repeated reference counted once, nested reference reports the outermost slot, both ends separated, slot recorded).
- [x] `INDEX-MUT` - insert/update/remove coherence
  - Proof: tests/mutation_edit.rs remove_deletes_the_entity_and_its_type_index_entry, remove_on_a_missing_id_returns_none_and_touches_nothing, remove_leaves_referrers_dangling_rather_than_rewriting_them; replacement coherence in tests/type_index_consistency.rs.
- [ ] `INDEX-PERF` - memory/build/query baselines
  - Proof: unit/property/adversarial tests plus crate clippy.

## Completion log

Append `TASK-ID - proof - material decision`; no long logs.
