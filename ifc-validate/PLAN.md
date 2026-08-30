# ifc-validate implementation plan

Status: implemented; structure, types, and natively checkable rules are evaluated,
and rules requiring an EXPRESS evaluator are reported as unsupported.
Last updated: 2026-08-30

This is task state, not ambient context. Follow `AGENTS.md`. Claim one task ID,
record blockers here, and check it off only with the stated evidence.

## Established boundary

Validate a Model against schema structure and registered semantic rules; never parse files itself.

## Planned file map

These paths are compiled private scaffold modules. Implement inside the named
owner and expose a public symbol only through an intentional parent re-export.

- `src/structure/reference.rs`: dangling/wrong-kind references
- `src/structure/cardinality.rs`: aggregate and optionality checks
- `src/structure/required.rs`: required attribute presence
- `src/structure/unique.rs`: UNIQUE rules and duplicate GUID reporting
- `src/type_check/entity.rs`: entity/subtype compatibility
- `src/type_check/select.rs`: SELECT membership
- `src/type_check/defined.rs`: defined/enumeration validation
- `src/type_check/enumeration.rs`: enumeration membership
- `src/type_check/scalar.rs`: scalar value form validation
- `src/where_rule/engine.rs`: bounded rule evaluation
- `src/where_rule/registry.rs`: explicit supported-rule registry
- `src/where_rule/builtin.rs`: audited native rule implementations
- `src/where_rule/budget.rs`: bounded evaluation and unsupported-rule limits
- `src/report/finding.rs`: structured diagnostics
- `src/report/summary.rs`: deterministic counts
- `src/report/path.rs`: entity/attribute source paths

## Work queue

- [x] `VAL-DEPS` - remove production dependency on ifc-step
  - Evidence: targeted tests plus crate clippy; add a focused fixture/property test.
- [x] `VAL-STRUCT` - implement reference/cardinality checks from schema metadata
  - Evidence: targeted tests plus crate clippy; add a focused fixture/property test.
- [x] `VAL-TYPE` - implement entity/select/defined-type compatibility
  - Evidence: targeted tests plus crate clippy; add a focused fixture/property test.
- [x] `VAL-WHERE` - register supported rules and report unsupported ones honestly
  - Evidence: targeted tests plus crate clippy; add a focused fixture/property test.
- [x] `VAL-REPORT` - deterministic reports with source paths and limits
  - Evidence: targeted tests plus crate clippy; add a focused fixture/property test.

## Completion log

Append concise entries as `TASK-ID - proof command/result - material decision`.
Do not paste long logs or transient process state.

- `VAL-DEPS` - removed `ifc-step`; focused crate check and package architecture
  gate pass, and a deliberate reintroduction is caught.
- `VAL-STRUCT` - 27 crate tests plus clippy - dangling/wrong-kind references,
  required slots, and scalar-vs-aggregate shape. Aggregate *defined types*
  (`IfcComplexNumber = ARRAY [1:2] OF REAL`) and typed wrappers over SELECT
  slots both had to be resolved before judging shape; a naive check produced a
  false positive on the corpus `pass-complex-number-ifc4.ifc`.
- `VAL-TYPE` - unknown entity types, abstract instantiation, and scalar form
  against the declared type. Catches `IFCPOSITIVELENGTHMEASURE('1')`.
- `VAL-WHERE` - three natively checkable rules implemented; the rest are
  registered as unsupported with a reason. A test pins the registry's
  `Implemented` claims to the engine's dispatch list, which immediately caught
  a duplicate `IfcRoot.WR1` entry for a check already covered file-wide.
- `VAL-REPORT` - severity/path/summary with a findings budget. `Unsupported`
  does not affect conformance; a truncated report says so.

## Deliberate gaps

- Aggregate *bounds* (`LIST [3:?]`) are not checked: the EXPRESS parser records
  that an attribute is an aggregate, not its bounds. Registered as unsupported
  rather than silently skipped.
- Arbitrary `WHERE` expressions need an EXPRESS evaluator, which this crate
  does not have.
- Header arity/type defects are not re-derived: `Model`'s header is normalized,
  so that evidence belongs to the codec's diagnostic channel.
