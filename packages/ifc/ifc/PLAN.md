# ifc implementation plan

Status: working feature facade with narrow cross-domain read adapters.
Last updated: 2026-08-20

This is task state, not ambient context. Follow `AGENTS.md`. Claim one task ID,
record blockers here, and check it off only with the stated evidence.

## Established boundary

Feature-gated convenience facade; it owns no records, codecs, or geometry
implementation. Cross-domain joins stay read-only and live here only when leaf
dependency rules prohibit them.

## Planned file map

Implemented owners:

- `src/io.rs`: codec discovery and path loading
- `src/feature_report.rs`: compiled feature diagnostics
- `src/material_templates.rs`: material-to-PSD query adapter
- `tests/features.rs`: compile/runtime feature surface assertions

## Work queue

- [x] `FACADE-MAP` - keep features aligned with implemented crate capabilities
  - Evidence: targeted tests plus crate clippy; add a focused fixture/property test.
- [x] `FACADE-MATERIAL-TEMPLATES` - expose material PSD joins without a leaf dependency edge
  - Evidence: `material_templates` and `material_template_inventory` tests plus package architecture gate.
- [x] `FACADE-SPLIT` - keep `lib.rs` declarative after adding an adapter module
  - Evidence: `no_monolithic_files`, facade all-feature tests, clippy, and rustdoc.
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
- `FACADE-MATERIAL-TEMPLATES` - joined the 14 official IFC4 material PSDs with explicit category policy and material-scoped exact-name lookup.
- `FACADE-SPLIT` - moved existing codec/path and feature-report behavior out of `lib.rs`; no behavior change.
