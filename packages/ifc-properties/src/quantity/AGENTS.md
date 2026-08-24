# ifc-properties quantity instructions

Scope: authored quantity views, validation, and transactional edits. Follow the crate `../../AGENTS.md`. Read `PLAN.md` only for
assigned task(s) `PROP-QTY, PROP-EDIT, PROP-CHECK` and keep implementation state there.

## Owns

- IfcElementQuantity and physical quantity forms
- value/unit/formula semantics
- edits from externally supplied typed measurements

## Does not own

- calling geometry
- recomputing volume/area/length
- silently replacing authored values

## Growth map

`set.rs`, `simple.rs`, `complex.rs`, `validation.rs`, `edit.rs`. These source owners already compile as private scaffold modules. Replace a module's planned-owner marker with its first real contract and tests; do not add parallel placeholders. Views borrow `ifc-model`; mutation waits
for an explicit model transaction contract.
