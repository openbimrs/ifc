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

- [x] `XML-NS` - add strict namespace/version-profile handling
  - Evidence: 22 crate tests/doc-tests, strict crate clippy, and 5/5 focused
    semantic mutants killed; explicit IFC4 ADD2 TC1 mode enforces the official
    local XSD namespace on every element,
    the root `IFC4` token, and strict output metadata while compatibility mode
    remains explicit for the existing schema-less dialect.
- [ ] `XML-VALUE` - extract a symmetric scalar contract shared by this codec reader/writer
  - Evidence: targeted tests plus crate clippy; add a focused fixture/property test.
- [ ] `XML-XSD` - validate generated fixtures against official XSD outside normal builds
  - Evidence: targeted tests plus crate clippy; add a focused fixture/property test.
- [ ] `XML-DIFF` - differential STEP to XML to Model corpus proof
  - Evidence: targeted tests plus crate clippy; add a focused fixture/property test.
- [x] `XML-DIAG` - preserve entity/attribute path in errors
  - Evidence: 22 crate tests/doc-tests, strict crate clippy, and 5/5 focused
    semantic mutants killed; non-finite writes, malformed XML, invalid entity ids, nested
    integer/real/reference scalars, list indexes, and unknown explicit value
    kinds retain an inspectable `XmlPath` and typed root
    cause instead of silently degrading invalid typed values.

## Completion log

Append concise entries as `TASK-ID - proof command/result - material decision`.

- `XML-NS` - `cargo +1.88.0 test -p ifc-xml` plus strict crate clippy;
  explicit IFC4 ADD2 TC1 mode resolves and enforces the official namespace on
  every element without relabelling compatibility-mode documents.
- `XML-DIAG` - the same crate gate covers typed `XmlPath` diagnostics through
  nested list/typed values and malformed XML; invalid typed scalars no longer
  degrade silently to null or text.
Do not paste long logs or transient process state.
