# ifc-properties pset instructions

Scope: property sets and all property value forms. Follow the crate `../../AGENTS.md`. Read `PLAN.md` only for
assigned task(s) `PROP-PSET` and keep implementation state there.

## Owns

- set identity/assignment links
- single, bounded, list, enumerated, table, reference, complex values

## Does not own

- quantity computation
- template authoring policy
- external dictionary I/O

## Growth map

`set.rs`, `scalar.rs`, `aggregate.rs`, `table.rs`, `reference.rs`, `complex.rs`. These source owners already compile as private scaffold modules. Replace a module's planned-owner marker with its first real contract and tests; do not add parallel placeholders. Views borrow `ifc-model`; mutation waits
for an explicit model transaction contract.
