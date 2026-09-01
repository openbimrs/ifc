# ifc-properties instructions

Purpose: Borrowed property, quantity, unit, template, and standard-library projections plus model authoring ports.

Follow `../AGENTS.md`. Read `PLAN.md` only for assigned implementation or
roadmap work; keep progress, blockers, and evidence there.

## Boundary

Allowed production dependencies: ifc-model and schema metadata; no geometry crate or backend.

## Module ownership

- `pset.rs`: property sets and property value forms
- `quantity.rs`: authored physical quantities and element quantity sets
- `unit.rs`: SI, conversion-based, derived, monetary, and unit assignment
- `template.rs`: property/quantity templates
- `standard.rs`: external property-set dictionaries
- `query.rs`: permissive assignments and bounded lookup for interactive clients
- `exact.rs`: fail-closed request-scoped IFC4 property resolution for rule engines
- `value.rs`: semantic conversion from generic Value

## Invariants

- An IFC quantity is an authored assertion. This crate reads/writes it; it never computes shape measurements.
- Applications compute via a geometry service and pass the resulting typed value into authoring APIs.
- Units stay explicit; no bare f64 crosses a public quantity authoring boundary.
- `exact_property` may return `Absent` only after complete schema-qualified assignment traversal; diagnostics, malformed references, ambiguity, unsupported values, and non-finite numbers are errors.
- `IfcRelDefinesByProperties.RelatingPropertyDefinition` is a select: traverse one definition reference or every member of a nonempty `IfcPropertySetDefinitionSet`; never treat an empty/malformed aggregate as absence.
- `IfcTypeObject.HasPropertySets` may be absent (`$`), but if present its `SET [1:?]` must be nonempty; present-empty or malformed aggregates are incomplete evidence.
- Every traversed EXPRESS `SET` rejects duplicate entity references; relationship targets and property-definition/property members are checked against their IFC4 entity domains before resolution.
- Every traversed entity record has exactly the concrete IFC4 schema arity; both missing and surplus Part 21 slots are incomplete evidence.
- `IfcPropertySingleValue` requires all four positional slots; present typed values must be recursively accepted by `IfcValue` **and** match their defined-type payload base, and present units must resolve through `IfcUnit` with exact concrete arity.
- Exact scalar values preserve their declared IFC value type and explicit unit identity. `IfcLogical` remains three-state and `IfcBinary` retains its payload; downstream adapters must reject categories or units they cannot project without loss.
- Keep permissive query APIs for interactive inspection; rule engines use the exact API and must not convert its errors to absence.

Keep cross-resource projections attribute-scoped: shared `ifc-model` storage
does not make one feature crate the owner of an IFC entity. Split typed views,
resolution, lowering, mutation, and validation before they grow together.

## Verification

Run targeted tests/clippy, isolated build, and the package architecture/context
gates. Geometry bridges also run declaration/corpus coverage and the full gate.
