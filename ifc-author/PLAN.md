# ifc-author implementation plan

Status: schema-checked construction and existing-entity editing implemented; OwnerHistory policy remains open. Last updated: 2026-08-31.
Follow `AGENTS.md`; claim one task and record blockers/decisions beneath it.

## Planned file map

These paths are implemented modules, not scaffold: each owns a tested contract
and is re-exported deliberately by its parent.

- `src/builder/entity.rs`: named-attribute builder and slot resolution
- `src/editor/entity.rs`: projected-state validation and transaction staging
- `src/check/declared.rs`: declared-type admissibility
- `src/error.rs`: refusal reasons

## Work queue

- [x] `AUTHOR-BUILD` - named-attribute builder resolving schema slot order
  - Proof: `cargo test -p ifc-author` (15 tests incl. doctest); mutation probe
    reversed the declared-attribute order and the suite failed.
- [x] `AUTHOR-CHECK` - arity, required, duplicate, type and aggregate refusal
  - Proof: 7 mutation probes disabling each check individually, all caught.
- [x] `AUTHOR-REAL` - authoring against the normative IFC4 schema
  - Proof: `cargo test -p ifc-author --test real_schema`; authored assembly
    round-trips through `ifc-step`.
- [x] `AUTHOR-EDIT` - update an entity already in a model, checked the same way
  - Evidence: 26 crate tests/doc-tests, strict crate clippy, and 3/3 focused
    semantic mutants killed; projected arity, required/type/aggregate/name
    checks stage nothing on refusal, while transaction preflight retains
    dangling-reference and revision atomicity.
- [ ] `AUTHOR-OWNERHISTORY` - derive `IfcOwnerHistory` for authored roots
  - Needs a decision on whether owner history is authored or injected by an
    application service; recorded in the roadmap, not settled here.

## Completion log

`AUTHOR-BUILD` - 15 tests, 8/8 mutants caught - schema tables drive slot order;
no generated per-entity structs, per ADR 0007.

`AUTHOR-EDIT` - `cargo +1.88.0 test -p ifc-author` and strict crate clippy -
`EntityEditor` validates the full projected record and stages positional edits
through `ifc-model::Transaction`; it does not create a second apply path.
