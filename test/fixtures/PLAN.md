# IFC test fixtures plan

Status: curated standalone fixture corpus used by IFC codec, validation, and geometry tests.
Last updated: 2026-08-25

This is task state, not ambient context. Follow `AGENTS.md` and load this file
only when changing the fixture corpus.

## Established boundary

Small synthetic or minimal upstream IFC files with preserved provenance.

## Planned file map

- `ifclite-geometry/`: geometry and processing edge cases.
- `ifcopenshell-validate/`: schema/header validation cases.
- `costing/`: local costing round-trip fixture.

## Work queue

- [ ] `FIXTURE-PROVENANCE` - add a machine-checkable source and license manifest for every fixture
  - Evidence: manifest coverage test maps every `.ifc` file to provenance and license metadata.

## Completion log

Append concise entries as `TASK-ID - proof command/result - material decision`.
