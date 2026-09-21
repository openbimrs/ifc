# ifc-element-type implementation plan

Status: complete; the catalogue and its writer are implemented.
Last updated: 2026-09-20

This is task state, not ambient context. Follow `AGENTS.md`; claim one task ID,
record blockers/decisions under it, and check it off only with evidence.

## Established boundary

Authoring the concrete `IfcTypeObject` subtypes: element, resource, and
process type definitions. Occurrence products stay with their own domain
crate; this crate writes only the type definition an occurrence points at.

## Planned file map

These paths are compiled private scaffold modules. Implement inside the named
owner and expose a public symbol only through an intentional parent re-export.

- `src/table/mod.rs`: generated catalogue root, `Family`, `ALL`
- `src/table/a_c.rs`: generated entries, IfcA* through IfcC*
- `src/table/d_f.rs`: generated entries, IfcD* through IfcF*
- `src/table/g_k.rs`: generated entries, IfcG* through IfcK*
- `src/table/l_p.rs`: generated entries, IfcL* through IfcP*
- `src/table/q_s.rs`: generated entries, IfcQ* through IfcS*
- `src/table/t_z.rs`: generated entries, IfcT* through IfcZ*
- `src/authoring.rs`: the writer and its refusals

## Work queue

- [x] `ETYPE-TABLE` - generate the catalogue from the EXPRESS schema
  - Evidence: `scripts/gen-element-types.py` emits 132 types across six
    shards; `the_catalogue_holds_its_invariants` asserts arity, slot and
    enum-membership properties over every entry.
- [x] `ETYPE-WRITE` - implement the type-definition writer
  - Evidence: `cargo test -p ifc-element-type` (7 tests), 10/10 mutation
    probes, crate clippy clean.

## Notes

The generator is the source of truth for `src/table/`. Regenerate with
`python3 scripts/gen-element-types.py` after a schema bump; never hand-edit
a shard.

Shards are sized against the post-rustfmt line count. rustfmt expands the
emitted constants to roughly 1.7x, and the monolith gate measures the
formatted file, so the shard count is chosen to leave headroom.

## Completion log

Append concise entries as `TASK-ID - proof command/result - material decision`.
Do not paste long logs or duplicate standing rules from `AGENTS.md`.

- `ETYPE-TABLE` - `cargo test -p ifc-element-type` 7 passed - The catalogue
  is GENERATED, not hand-written: 132 types with 1166 enum members is past
  the size where a typo stays invisible. `PredefinedType` sits at slot 9 for
  125 types, 10 for `IfcFurnitureType`, and 11 for the six resource types,
  whose supertype interposes `BaseCosts` and `BaseQuantity` - so the slot is
  per-entry data, never a constant. The USERDEFINED fallback is the mirror
  case: it is ALWAYS slot 8, because `ElementType`, `ResourceType` and
  `ProcessType` are each the last attribute their own supertype adds. Slots 6
  and 7 diverge by family (`RepresentationMaps`/`Tag` vs
  `Identification`/`LongDescription`), which is why `Family` is stored rather
  than inferred from the type name.
- `ETYPE-WRITE` - `cargo test -p ifc-element-type` 7 passed; 10/10 mutation
  probes - `CorrectPredefinedType` is enforced at construction: a token must
  belong to that entity's own enum, and USERDEFINED is refused unless the
  fallback name is present and non-blank. A blank string is refused too; it
  satisfies EXISTS in EXPRESS but names nothing, so accepting it would emit a
  file that passes a validator and still means nothing. The writer takes a
  `&ElementType` from the catalogue rather than a type-name string, so an
  unknown entity cannot be spelled at all.
