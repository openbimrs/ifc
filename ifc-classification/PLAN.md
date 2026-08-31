# ifc-classification implementation plan

Status: implemented IFC4 classification/document/library views, queries, associations, and transactional authoring.
Last updated: 2026-08-31

This is task state, not ambient context. Follow `AGENTS.md`; claim one task ID,
record blockers/decisions under it, and check it off only with evidence.

## Established boundary

Borrowed IFC4 classification, document, library, and object-association
projections. Bounded classification hierarchy and explicit occurrence/type
lookup never silently merge sources. Authoring stages records through
`ifc_model::Transaction` and performs domain validation before staging.

Generic external-reference/resource relationships outside these nine concrete
records remain separate future capability and are not implied by this plan.

## Planned file map

These paths began as compiled private scaffold modules and now own the implemented
capability:
- `src/view.rs`: strict shared slot/value decoding and borrowed view facade
- `src/classification/system.rs`: `IfcClassification`
- `src/classification/reference.rs`: hierarchical `IfcClassificationReference`
- `src/document/information.rs`: `IfcDocumentInformation`
- `src/document/reference.rs`: `IfcDocumentReference`
- `src/library/information.rs`: `IfcLibraryInformation`
- `src/library/reference.rs`: `IfcLibraryReference`
- `src/assignment/classification.rs`: object classification links
- `src/assignment/document.rs`: document associations
- `src/assignment/library.rs`: library associations
- `src/query/hierarchy.rs`: bounded hierarchy and explicit occurrence/type lookup
- `src/authoring.rs`: transaction-staged creation for all owned concrete records
- `tests/classification.rs`: schema layout, view, query, invalid-input, authoring, and rollback proof

## Work queue

- [x] `CLASS-SYS` - implement classification systems/references
- [x] `CLASS-DOC` - implement document information/references
- [x] `CLASS-LIB` - implement library information/references
- [x] `CLASS-ASSIGN` - implement classification/document/library association views
- [x] `CLASS-QUERY` - define bounded hierarchy and explicit occurrence/type semantics
- [x] `CLASS-MUT` - add transaction-staged authoring after `MODEL-MUT`

## Completion log

- `CLASS-SYS` / `CLASS-DOC` / `CLASS-LIB` - bundled IFC4 layout assertions pin every interpreted inherited-first slot; strict borrowed accessors and WHERE-rule validation pass focused tests.
- `CLASS-ASSIGN` / `CLASS-QUERY` - deterministic relationship queries, bounded hierarchy traversal, cycle/dangling/wrong-type failures, and explicit occurrence/type separation pass focused tests.
- `CLASS-MUT` - all nine owned concrete records stage through `Transaction`; invalid GUID, external identity, document XOR, actor/select targets, duplicate SET members, `IfcDefinitionSelect`, and failed-commit rollback behavior pass focused tests.
- Gate proof: `cargo +1.88.0 test -p ifc-classification`; strict all-target clippy; 13/13 classification mutation probes killed.
