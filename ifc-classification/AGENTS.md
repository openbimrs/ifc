# ifc-classification instructions

Purpose: Borrowed IFC2X3/IFC4 classification, document, library, association,
hierarchy, and transaction-staged IFC4 authoring semantics.

Follow `../AGENTS.md`. Read `PLAN.md` only for assigned implementation or
roadmap work; keep progress, blockers, and evidence there.

## Boundary

Allowed production dependencies: `ifc-model`, `ifc-schema`, and shared error
support only. Views borrow the model. Authoring stages edits on a caller-owned
`ifc_model::Transaction`; this crate does not own commit or rollback.

The implemented boundary is ten concrete IFC4 records: classification,
classification reference, document information/reference, library
information/reference, the three `IfcRelAssociates*` families, and
`IfcExternalReferenceRelationship`. Approval and constraint resources remain
separate sibling domains.

## Module ownership

- `view.rs`: strict shared scalar/reference decoding and `ClassificationView`
- `release.rs`: the release a model binds (`classification_schema`) and name-based slot/select lookup
- `classification.rs`: systems, editions, hierarchical references, and IFC2X3 notations
- `document.rs`: document information/references
- `library.rs`: library information/references
- `assignment.rs`: classification/document/library object associations
- `external_relationship.rs`: generic external-reference resource relationships
- `query.rs`: bounded hierarchy and explicit occurrence/type lookup
- `authoring.rs`: transaction-staged creation and pre-staging validation
- `error.rs`: malformed, unresolved, ambiguous, budget, and authoring failures

## Invariants

- External URI/file/network access is never triggered by reading a view.
- Classification codes are identifiers, not numbers; preserve formatting and hierarchy.
- Occurrence and type associations are returned separately and never silently merged.
- Missing, malformed, dangling, wrong-kind, cyclic, ambiguous, or budget-exhausted input returns a typed failure.
- Interpreted inherited-first slot layouts are pinned against bundled IFC4 metadata.
- Reads bind to the release the header declares (`IFC2X3` → IFC2X3 table; anything else single or undeclared → IFC4, the 0.2.0 behaviour). Slots are looked up by attribute NAME in the bound table via `Release::slot`, never by a hard-coded position; the only aliases are IFC2X3 `ItemReference`/`DocumentId` ≙ IFC4 `Identification`. Selects/domains come from the bound attribute's declared type (`Release::accepts`), never a hand-written type list, and never `ifc_schema::ifc4()` on a read path.
- An attribute or record the bound release lacks is `NotInSchema`, never `Ok(None)`; a record-typed value behind a text accessor is `StructuredValue`, never `InvalidValue`.
- Authoring validates the complete draft before its first staged edit.
- Iteration and relationship query output is deterministic by entity ID.

Keep entity views, relationship traversal, mutation, and domain algorithms in
separate files. Public exports flow through `lib.rs` deliberately.

## Verification

Run `cargo +1.88.0 test -p ifc-classification`, strict all-target clippy, the
classification mutation harness, then the package architecture/context/full
gates. Relationship changes require cycle, dangling, wrong-type, ambiguity,
budget, and rollback cases as applicable.
