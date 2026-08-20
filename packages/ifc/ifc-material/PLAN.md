# ifc-material implementation plan

Status: IFC4 MaterialResource read/query support implemented; authoring and geometry cross-proof remain.
Last updated: 2026-08-20

This is task state, not ambient context. Follow `AGENTS.md`; claim one task ID,
record blockers/decisions under it, and check it off only with evidence.

## Established boundary

Borrowed semantic projections for materials, layers, profiles, constituents, and their usage/assignment.

## Planned file map

These paths own the implemented borrowed views. Extend the named owner and
expose public symbols only through an intentional parent re-export.

- `src/material/definition.rs`: IfcMaterial identity
- `src/material/properties.rs`: material property relationships
- `src/layer/definition.rs`: identity, material link, metadata, and authored thickness
- `src/layer/set.rs`: ordered layer sets
- `src/layer/usage.rs`: semantic association to a layer set only
- `src/profile/definition.rs`: material/name/description/priority/category
- `src/profile/set.rs`: ordered profile sets
- `src/profile/usage.rs`: semantic association to a profile set only
- `src/constituent/definition.rs`: constituent semantics
- `src/constituent/set.rs`: set membership
- `src/usage/assignment.rs`: RelAssociatesMaterial view
- `src/usage/resolution.rs`: bounded association resolution

- `src/material/relationships.rs`: lists, classification links, and resource relationships

## Active implementation: IFC4 MaterialResource

Goal: provide application-facing, borrowed, codec-independent views for every
IFC4 ADD2 TC1 declaration in MaterialResource, including assignment resolution,
material property sets, all 18 entities, four declared types, and
`IfcMlsTotalThickness`.

Decision: material owns authored MaterialResource data; geometry owns geometric
interpretation and lowering. Unknown or malformed values stay explicit.

## Work queue

- [x] `MAT-SPEC` - pin the normative declaration and slot inventory in executable tests
  - Evidence: IFC4 declaration matrix covers 18 entities, four types, and one function.
- [x] `MAT-BASE` - implement material identity, relationships, lists, and properties
  - Evidence: focused unit/property/fixture tests, isolated build, and crate clippy.
- [x] `MAT-LAYER` - implement layer composition and authored usage fields
  - Evidence: focused unit/property/fixture tests, isolated build, and crate clippy.
- [x] `MAT-PROFILE` - implement profile composition and authored usage fields
  - Evidence: focused unit/property/fixture tests, isolated build, and crate clippy.
- [x] `MAT-CONST` - implement constituents and fraction validation
  - Evidence: focused unit/property/fixture tests, isolated build, and crate clippy.
- [x] `MAT-ASSIGN` - resolve product/type material associations deterministically
  - Evidence: focused unit/property/fixture tests, isolated build, and crate clippy.
- [x] `MAT-PSET` - join `IfcMaterialProperties` instances to the 14 official PSD templates
  - Evidence: exact-name/entity applicability plus explicit category-policy tests through the `ifc` facade.
- [ ] `MAT-MUT` - add authoring only after MODEL-MUT exists
  - Evidence: focused unit/property/fixture tests, isolated build, and crate clippy.
- [ ] `MAT-CROSS` - prove material and geometry projections join by EntityId without duplicate slot ownership
  - Requires: `MAT-LAYER`, `MAT-PROFILE`, `INPUT-MAT`.
  - Evidence: cross-projection fixtures, isolated build, and crate clippy.

## Completion log

Append concise entries as `TASK-ID - proof command/result - material decision`.
Do not paste long logs or move standing invariants out of `AGENTS.md`.

- `MAT-SPEC`..`MAT-ASSIGN` - all-feature crate tests and IFC STEP facade fixture pass.
- `MAT-PSET` - facade tests pin all 14 material PSDs and exact-name lookup.
- Review hardening - strict aggregate/required-slot decoding, bounded typed wrappers,
  finite thickness sums, duplicate type-relation ambiguity, and exact IFC4 type-target validation.
