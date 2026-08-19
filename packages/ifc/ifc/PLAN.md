# ifc implementation plan

Status: working feature facade; feature matrix must track new domain capabilities.
Last updated: 2026-08-19

This is task state, not ambient context. Follow `AGENTS.md`. Claim one task ID,
record blockers here, and check it off only with the stated evidence.

## Established boundary

Feature-gated convenience facade; it owns no records, semantics, codec, or geometry implementation.

## Planned file map

These paths already compile as private scaffold owners. Replace a planned-owner
marker with its first real contract and tests; do not add parallel placeholders.

- `src/prelude.rs`: only if repeated imports justify a small stable prelude
- `tests/features.rs`: compile/runtime feature surface assertions

## Work queue

- [x] `FACADE-MAP` - keep features aligned with implemented crate capabilities
  - Evidence: targeted tests plus crate clippy; add a focused fixture/property test.
- [ ] `FACADE-LEAN` - measure cargo tree for no-default and individual features
  - Evidence: targeted tests plus crate clippy; add a focused fixture/property test.
- [ ] `FACADE-DOC` - document capability bundles without hiding leaf crates
  - Evidence: targeted tests plus crate clippy; add a focused fixture/property test.
- [ ] `FACADE-JSON` - add an IFC-JSON feature only after a real codec crate exists
  - Evidence: targeted tests plus crate clippy; add a focused fixture/property test.

## Completion log

Append concise entries as `TASK-ID - proof command/result - material decision`.
Do not paste long logs or transient process state.

- `FACADE-MAP` - added isolated `migrate` and `infer` features plus `full` bundle wiring; default and combined-feature facade tests and doc-tests pass.
