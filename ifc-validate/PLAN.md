# ifc-validate implementation plan

Status: implemented for structural/type validation and selected native rules across
bundled IFC2X3 TC1, IFC4 ADD2 TC1, and IFC4X3 ADD2 tables. General EXPRESS
expressions, aggregate bounds, and inverse semantics remain explicitly unsupported.
Last updated: 2026-08-31

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
- [x] `VAL-4X3` - validate declared IFC4X3 ADD2 models against their bundled tables
  - Evidence: valid/invalid committed fixtures, exact schema selection, no IFC4 fallback, corpus audit, mutation probes, and full gate.
- [x] `VAL-RULES-2` - implement selected direct-value/reference rules without claiming general EXPRESS support
  - Evidence: positive/negative tests across applicable schema versions; aggregate-bound, arbitrary-WHERE, and INVERSE gaps remain explicit.
- [x] `VAL-REPORT` - deterministic reports with source paths and limits
  - Evidence: sorting/path/summary tests; `max_findings` hard-caps every
    `Report::push`/`extend` (regression: budget 5 previously stored 100 findings).

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
- `VAL-WHERE` - nine registered implemented rule IDs cover eight native checks;
  version-labelled sequence rules share one implementation. The registry keeps
  aggregate-bound, general-expression, geometry, and inverse-dependent rules
  explicit as unsupported and is pinned to dispatch by test.
- `VAL-4X3` - IFC4X3 ADD2 declared files use their independent 876-entity,
  436-type bundle. Existing valid and malformed committed fixtures now validate
  or fail rather than being skipped; IFC4 tables are never substituted.
- `VAL-RULES-2` - added external-reference identity, sequence endpoint,
  decomposition/nesting self-reference, and material-layer priority checks with
  schema-derived attribute positions and version guards.
- `VAL-REPORT` - severity/path/summary with a hard findings-storage cap.
  `Unsupported` does not affect conformance; a truncated report says so.

## Deliberate gaps

- Aggregate *bounds* (`LIST [3:?]`) are not checked: the EXPRESS parser records
  that an attribute is an aggregate, not its bounds. Registered as unsupported
  rather than silently skipped.
- Arbitrary `WHERE` expressions need an EXPRESS evaluator, which this crate
  does not have.
- INVERSE relationship semantics are not derived. `IfcDocumentReference.WR1`
  remains registered as unsupported because its IFC2X3 form depends on an
  inverse even though later schemas expose a direct reference.
- Header arity/type defects are not re-derived: `Model`'s header is normalized,
  so that evidence belongs to the codec's diagnostic channel.

## Corpus audit, 2026-08-31

Ran the validator over all 38 committed fixtures. Results: 31 clean and 7
expected/known-invalid fixtures reported errors; none were skipped for schema
coverage. IFC2X3 TC1, IFC4 ADD2 TC1, and IFC4X3 ADD2 fixtures are each validated
against their independent bundled tables rather than skipped or forced through
another version. Two of the 31 model-level-clean fixtures are raw-header-only
failures whose evidence the codec normalizes before validation.

The run exposed two validator bugs before it exposed any fixture bug:

- SELECT membership was bounded by loop iterations, not visited types, so wide
  IFC4 value selects returned false negatives. 8 false findings.
- The first audit harness forced IFC4 tables on every file, inventing 20
  slot-count errors in IFC2X3 fixtures. `validate_declared` exists precisely to
  refuse that; the harness now uses it.

Genuine fixture defects, both independently confirmed by `ifcopenshell.validate`:

- `nurbs/ifc4_rational_bspline_curve_surface.ifc` instantiated two abstract
  entities. Split into a separate invalid fixture.
- `ifclite-geometry/nested_mapped_item_cycle.ifc` omits a mandatory
  `MappingTarget`. Already deliberate and documented in `ifc-geometry`.
