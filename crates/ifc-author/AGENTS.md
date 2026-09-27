# ifc-author instructions

Scope: schema-checked construction and editing of IFC entities. Follow the package
`../../AGENTS.md`.

## Owns

- named-attribute construction resolved to STEP positional slots
- construction-time refusal: unknown entity/attribute, duplicate set, missing
  required, declared-type and aggregate mismatch, malformed GlobalId
- insertion of a built entity into a `Model`
- named-attribute edits validated against the complete projected entity before
  staging through `ifc-model::Transaction`
- explicit owner-history construction (`src/owner.rs`: person, organization,
  application, `IfcOwnerHistory`). Nothing attaches it automatically: a
  domain draft that accepts one takes it from the caller, because an invented
  actor is worse than a null

## Does not own

- WHERE rules, inverse attributes, uniqueness, cross-entity consistency
  (`ifc-validate` audits an existing model; this refuses a bad construction)
- serialization: an authored model is written by whichever codec the caller picked
- domain semantics: this crate knows no walls, styles, or annotations, only
  what the schema tables declare
- schema-agnostic mutation primitives, reference-integrity preflight, optimistic
  concurrency, and atomic commit (`ifc-model`'s `mutation` module)

## Boundaries

L2. Depends on `ifc-model` (L0) and `ifc-schema` (L1) and nothing else. Adding a
codec or a sibling domain dependency here inverts the tiers; `ifc-model` must
never gain a schema dependency to make authoring easier.

Value checking is deliberately permissive: an unresolvable declared type is
accepted rather than refused, because a builder that rejects valid input is
worse than one that misses an exotic mistake. Tighten `check/declared.rs` only
with a test that proves the newly-refused value is genuinely invalid.

## Pitfalls

- Attribute names match case-insensitively (`eq_ignore_ascii_case`), so an
  IFC4/IFC4X3 spelling that differs only in case is not a bug. IFC4X3 has
  exactly three true renames; the authored-coverage record counts them.
- SELECTs such as `IfcAxis2Placement` are not supertypes: `is_a` rejects their
  members. Use `accepts_type`, which resolves SELECTs and aliases.
- A `Transaction` cannot be read back. To assert on a staged entity, commit
  first or inspect `tx.edits()`.
- An empty aggregate is invalid for a `LIST [1:?]` / `SET [1:?]` attribute:
  write `$` when it is optional, refuse when it is required. This crate cannot
  check it, because the schema tables keep no aggregate bounds (#111).
