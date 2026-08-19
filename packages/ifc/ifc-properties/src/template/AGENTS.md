# ifc-properties template instructions

Scope: property/quantity templates and applicability relationships. Follow the crate `../../AGENTS.md`. Read `PLAN.md` only for
assigned task(s) `PROP-TEMPLATE` and keep implementation state there.

## Owns

- set/property template views
- applicability and template relationships
- declared measure/property forms

## Does not own

- generating UI forms
- enforcing product-specific business policy
- external catalog downloads

## Growth map

`property_set.rs`, `property.rs`, `relationship.rs`. These source owners already compile as private scaffold modules. Replace a module's planned-owner marker with its first real contract and tests; do not add parallel placeholders. Views borrow `ifc-model`; mutation waits
for an explicit model transaction contract.
