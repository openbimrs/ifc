# ifc-classification implementation plan

Status: architecture scaffold; projections and queries remain to implement.
Last updated: 2026-08-19

This is task state, not ambient context. Follow `AGENTS.md`; claim one task ID,
record blockers/decisions under it, and check it off only with evidence.

## Established boundary

Borrowed classification, document, library, and association projections.

## Planned file map

These paths are compiled private scaffold modules. Implement inside the named
owner and expose a public symbol only through an intentional parent re-export.

- `src/classification/system.rs`: IfcClassification
- `src/classification/reference.rs`: hierarchical references
- `src/document/information.rs`: document metadata
- `src/document/reference.rs`: document locations/identifiers
- `src/library/information.rs`: library metadata
- `src/library/reference.rs`: library entries
- `src/assignment/classification.rs`: object classification links
- `src/assignment/document.rs`: document associations
- `src/assignment/library.rs`: library associations
- `src/query/hierarchy.rs`: bounded parent/child traversal

## Work queue

- [ ] `CLASS-SYS` - implement classification systems/references
  - Evidence: focused view/query tests, invalid/cycle cases, and crate clippy.
- [ ] `CLASS-DOC` - implement document information/references
  - Evidence: focused view/query tests, invalid/cycle cases, and crate clippy.
- [ ] `CLASS-LIB` - implement library information/references
  - Evidence: focused view/query tests, invalid/cycle cases, and crate clippy.
- [ ] `CLASS-ASSIGN` - implement all association views
  - Evidence: focused view/query tests, invalid/cycle cases, and crate clippy.
- [ ] `CLASS-QUERY` - define occurrence/type/hierarchy lookup semantics
  - Evidence: focused view/query tests, invalid/cycle cases, and crate clippy.
- [ ] `CLASS-MUT` - add authoring only after MODEL-MUT
  - Evidence: focused view/query tests, invalid/cycle cases, and crate clippy.

## Completion log

Append concise entries as `TASK-ID - proof command/result - material decision`.
Do not paste long logs or duplicate standing rules from `AGENTS.md`.
