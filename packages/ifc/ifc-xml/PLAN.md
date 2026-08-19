# ifc-xml implementation plan

Status: working structural codec; namespace/XSD conformance and broader corpus coverage incomplete.
Last updated: 2026-08-19

This is task state, not ambient context. Follow `AGENTS.md`. Claim one task ID,
record blockers here, and check it off only with the stated evidence.

## Established boundary

ifcXML codec adapter between XML and ifc-model.

## Planned file map

These paths already compile as private scaffold owners. Replace a planned-owner
marker with its first real contract and tests; do not add parallel placeholders.

- `src/reader/namespace.rs`: namespace/profile handling
- `src/reader/entity.rs`: entity/reference decoding
- `src/writer/entity.rs`: named/positional attribute output
- `src/value/scalar.rs`: typed scalar conversion
- `src/value/aggregate.rs`: aggregate/select conversion

## Work queue

- [ ] `XML-NS` - implement strict namespace/version profile handling
  - Evidence: targeted tests plus crate clippy; add a focused fixture/property test.
- [ ] `XML-VALUE` - extract a symmetric scalar contract shared by this codec reader/writer
  - Evidence: targeted tests plus crate clippy; add a focused fixture/property test.
- [ ] `XML-XSD` - validate generated fixtures against official XSD outside normal builds
  - Evidence: targeted tests plus crate clippy; add a focused fixture/property test.
- [ ] `XML-DIFF` - differential STEP to XML to Model corpus proof
  - Evidence: targeted tests plus crate clippy; add a focused fixture/property test.
- [ ] `XML-DIAG` - preserve entity/attribute path in errors
  - Evidence: targeted tests plus crate clippy; add a focused fixture/property test.

## Completion log

Append concise entries as `TASK-ID - proof command/result - material decision`.
Do not paste long logs or transient process state.
