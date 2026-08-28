# ifc-model mutation plan

Status: planned under `MODEL-MUT`. Last updated: 2026-08-28.
Follow `AGENTS.md`; claim one task and record blockers/decisions beneath it.

## Work queue

- [x] `MUT-EDIT` - insert/update/remove/value edit operations
  - Proof: `set_attribute`, `set_attributes`, `retype`, `remove` on `Model`
    (`edit.rs`). Unit tests in `tests/mutation_edit.rs`, doctests, crate
    clippy clean.
- [ ] `MUT-PREFLIGHT` - ID/reference/index conflicts
  - Proof: unit/property/adversarial tests plus crate clippy.
- [ ] `MUT-COMMIT` - atomic apply/rollback semantics
  - Proof: unit/property/adversarial tests plus crate clippy.
- [ ] `MUT-PROP` - property tests for index consistency
  - Proof: unit/property/adversarial tests plus crate clippy.

## Completion log

Append `TASK-ID - proof - material decision`; no long logs.

- `MUT-EDIT` - `cargo test -p ifc-model`, `cargo clippy -p ifc-model` - added
  `entities_mut`/`by_type_mut`/`order_mut` crate-private seams on `Model`
  rather than a bare `get_mut`, because `type_name` is also a `by_type` key;
  a public `get_mut` would let a caller silently desync the index. Fixes
  https://github.com/openbimrs/ifc/issues/3.
