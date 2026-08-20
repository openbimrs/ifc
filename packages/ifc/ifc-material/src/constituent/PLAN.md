# ifc-material constituent plan

Status: IFC4 views implemented under `MAT-CONST`. Last updated: 2026-08-20.
Follow `AGENTS.md`; claim one task and record blockers/decisions beneath it.

## Work queue

- [x] `CONST-VIEW` - constituent definitions
  - Proof: `definitions::reads_material_identity_constituents_and_sets` plus crate clippy.
- [x] `CONST-SET` - set membership
  - Proof: `definitions::reads_material_identity_constituents_and_sets` pins source-order projection without claiming SET order.
- [ ] `CONST-RULE` - explicit fraction consistency diagnostics
  - Proof required: focused policy tests plus crate clippy.

## Completion log

`CONST-VIEW`, `CONST-SET` - `tests/definitions.rs`; fraction-sum diagnostics
remain policy work because IFC4 does not impose a sum WHERE rule.
