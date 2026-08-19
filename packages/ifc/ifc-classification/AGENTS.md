# ifc-classification instructions

Purpose: Borrowed classification, document, library, and association projections.

Follow `../AGENTS.md`. Read `PLAN.md` only for assigned implementation or
roadmap work; keep progress, blockers, and evidence there.

## Boundary

Allowed production dependencies: ifc-model and schema metadata only.

## Module ownership

- `classification.rs`: systems, editions, and references
- `document.rs`: document information/references
- `library.rs`: library information/references
- `assignment.rs`: object/type associations
- `query.rs`: bounded lookup and inheritance
- `error.rs`: unresolved/ambiguous references

## Invariants

- External URI/file/network access is never triggered by reading a view.
- Classification codes are identifiers, not numbers; preserve formatting and hierarchy.
- Occurrence/type association precedence is explicit and never silently merged.

Keep entity views, relationship traversal, mutation, and domain algorithms in
separate files. New child modules remain crate-private until a real public
contract is ready for deliberate re-export.

## Verification

Run targeted tests/clippy, then the package architecture/context gates. Add
fixtures and cycle/invalid-input cases for every relationship traversal.
