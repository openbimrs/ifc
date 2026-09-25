# ifc-systems instructions

Purpose: Borrowed building/distribution system, port, flow, zone, and semantic-connectivity projections.

Follow `../AGENTS.md`. Read `PLAN.md` only for assigned implementation or
roadmap work; keep progress, blockers, and evidence there.

## Owns

- `IfcBuildingSystem`, `IfcBuiltSystem`, and `IfcDistributionCircuit` authoring.
  The circuit swaps `LongName` and `PredefinedType` relative to the other two,
  so each carries its own slot pair rather than sharing one path.
  `IfcBuiltSystem` is IFC4X3 only; `IfcBuildingSystem` is its IFC4 predecessor
  and stays declared, though deprecated, in IFC4X3.

## Boundary

Allowed production dependencies: ifc-model and schema metadata only; no geometry/spatial algorithm crate.

## Module ownership

- `system.rs`: systems and distribution systems
- `port.rs`: ports and product nesting
- `connectivity.rs`: semantic port/element connections
- `flow.rs`: flow direction and role semantics
- `zone.rs`: zones/spatial groups
- `assignment.rs`: product/service/system links
- `query.rs`: bounded semantic graph traversal
- `error.rs`: malformed/cyclic system graphs

## Invariants

- System connectivity comes from IFC relationships, not geometric proximity.
- No pressure-flow solver, clash test, routing algorithm, or geometry import enters this crate.
- Direction conflicts and cycles are reported; traversal always has explicit budgets.
- Every read path binds to the release `FILE_SCHEMA` declares (`release::resolve`, public as `schema_of`); ancestry and slot presence come from that release's bundled table, never a hard-wired `ifc_schema::ifc4()`. Only authoring is IFC4-targeted by design.
- The bulk readers (`systems`, `zones`, `ports`, `ElementRole::of`, `ConnectionGraph::build`) keep their 0.2.0 signatures, so a header they cannot bind (none, several, or unsupported such as IFC4X3) reads against IFC4 as before; callers that need a refusal call `schema_of` first. Checked accessors (`long_name_of`) refuse with `SchemaGap` instead.
- An attribute the declared release lacks (IFC2X3 `IfcZone.LongName`) is reported by the checked accessor as `NotInSchema`, never as an authored-empty `None`.

Keep entity views, relationship traversal, mutation, and domain algorithms in
separate files. New child modules remain crate-private until a real public
contract is ready for deliberate re-export.

## Verification

Run targeted tests/clippy, then the package architecture/context gates. Add
fixtures and cycle/invalid-input cases for every relationship traversal.
