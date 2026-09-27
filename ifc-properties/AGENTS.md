# ifc-properties instructions

Purpose: Borrowed property, quantity, unit, template, and standard-library projections plus model authoring ports.

Follow `../AGENTS.md`.

## Boundary

Allowed production dependencies: ifc-model and schema metadata; no geometry crate or backend.

## Module ownership

- `pset.rs`: property sets and property value forms
- `quantity.rs`: authored physical quantities and element quantity sets
- `unit.rs`: SI, conversion-based, derived, monetary, and unit assignment
- `template.rs`: property/quantity templates
- `standard.rs`: external property-set dictionaries
- `query.rs`: permissive assignments and bounded lookup for interactive clients
- `exact.rs`: fail-closed request-scoped IFC2X3/IFC4/IFC4X3 property and unit resolution and property enumeration for rule engines
- `value.rs`: semantic conversion from generic Value
- `nesting.rs`: crate-private bounded, cycle-aware traversal of complex members

## Invariants

- An IFC quantity is an authored assertion. This crate reads/writes it; it never computes shape measurements.
- Applications compute via a geometry service and pass the resulting typed value into authoring APIs.
- Units stay explicit; no bare f64 crosses a public quantity authoring boundary.
- `exact_property` may return `Absent` only after complete schema-qualified assignment traversal; diagnostics, malformed references, ambiguity, unsupported values, and non-finite numbers are errors.
- `exact_property` binds to the one release `FILE_SCHEMA` declares (IFC2X3, IFC4 or IFC4X3; a release is added only after its differences are pinned by tests) and reads every structural fact from that release's bundled table (`ifc_schema::for_version`). Never hard-wire `ifc4()`; never alias names across releases. A construct the declared release does not define is `NotInSchema`, not a best-effort answer.
- `exact_properties`/`exact_properties_where` run the same traversal as `exact_property` and must stay equivalent to it for one selected set name and one property name (`tests/exact_enumeration.rs` compares them); an empty result is a proven absence. The set selector may skip a set unread, as a set name does; every member of a selected set is validated, and only selected members must resolve.
- `IfcRelDefinesByProperties.RelatingPropertyDefinition` is a select in IFC4 and IFC4X3: traverse one definition reference or every member of a nonempty `IfcPropertySetDefinitionSet`; never treat an empty/malformed aggregate as absence. In IFC2X3 it is a single reference, and a set there is foreign.
- `IfcTypeObject.HasPropertySets` may be absent (`$`), but if present its `SET [1:?]` must be nonempty; present-empty or malformed aggregates are incomplete evidence.
- Every traversed EXPRESS `SET` rejects duplicate entity references; relationship targets and property-definition/property members are checked against the declared release's entity domains before resolution.
- Every traversed entity record has exactly the declared release's concrete arity; both missing and surplus Part 21 slots are incomplete evidence.
- A proper subtype of `IfcRelDefinesByProperties`/`IfcRelDefinesByType` relating the queried object (IFC2X3 `IfcRelOverridesProperties`) is refused, never skipped: `Model::ids_of_type` is exact-type.
- `IfcPropertySingleValue` requires all four positional slots; present typed values must be recursively accepted by `IfcValue` **and** match their defined-type payload base, and present units must resolve through `IfcUnit` with exact concrete arity.
- The other `IfcSimpleProperty` kinds (enumerated, list, bounded, table, reference) are read by attribute name from the bound release's table (`exact/composite.rs`): only a slot that release declares `OPTIONAL` may be `$`, a slot it does not declare is never read (IFC2X3 has no `SetPointValue`/`CurveInterpolation`), each list member is an `IfcValue` accepted as a `NominalValue` is, and a reference target must be accepted by the attribute's declared type with exact arity. The WHERE rules that decide how the values read (one type per list and between bounds, equal table columns, selected values drawn from the referenced enumeration) and `LIST OF UNIQUE` are enforced as `InconsistentValues` naming the release's rule label; other WHERE rules on values stay unevaluated.
- Exact scalar values preserve their declared IFC value type and explicit unit identity. `IfcLogical` remains three-state and `IfcBinary` retains its payload; downstream adapters must reject categories or units they cannot project without loss.
- `exact_property` searches quantity sets like property sets. It resolves
  simple quantities with the release's declared value measure. It never
  skips an `IfcPropertySetDefinition` that could hold the requested name.
- A predefined set's members are the attributes its entity declares below
  `IfcPropertySetDefinition`, read by name from the bound release's table
  (`exact/predefined.rs`) and typed by their declaration: defined and
  simple types as scalars, enumerations against the release's members,
  selects as the member written, entities as checked unfollowed references.
  `$` is `Null` only where `OPTIONAL`; an aggregate attribute is refused
  when selected. Its set name is `Name`, or its entity name when it states
  none. An unnamed set outside the set selection still refuses when one of
  its attributes is selected (it cannot be ruled out by name, #66), and
  unnamed sets of one entity are ambiguous only for a member they share, so
  a door with one `IfcDoorPanelProperties` per leaf stays answerable;
  `exact_predefined_sets` lists them all without applying an override.
- `exact_unit` binds to the same release as `exact_property`. SI and unit-type dimensions come from that release's `IfcDimensionsForSiUnit`/`IfcCorrectDimensions` (IFC2X3 differs from IFC4/IFC4X3 for the farad). A measure maps to a unit type only by the release's own enums and defined-type chain. Unknown prefixes, duplicate project units, cycles, dimension contradictions and offset units are errors, never defaults.
- Nested `IfcComplexProperty`/`IfcPhysicalComplexQuantity` members are read
  only through `src/nesting.rs`: a tracked path (cycles of any length, not
  just the direct self-member the schema forbids), a depth bound and a member
  budget. Shared members are legal and are not cycles. Every cut, and every
  member the permissive views cannot represent (absent id, non-reference
  list item, repeated member, valueless or non-numeric quantity), is a
  `PropertyAnomaly`, never a silent omission. Member lists of sets and
  complex entities are read only through `Nesting::members`.
- Keep permissive query APIs for interactive inspection; rule engines use the exact API and must not convert its errors to absence.

Keep cross-resource projections attribute-scoped: shared `ifc-model` storage
does not make one feature crate the owner of an IFC entity. Split typed views,
resolution, lowering, mutation, and validation before they grow together.

## Verification

Run targeted tests/clippy, isolated build, and the package architecture/context
gates. Geometry bridges also run declaration/corpus coverage and the full gate.
