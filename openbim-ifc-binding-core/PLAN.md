# openbim-ifc-binding-core implementation plan

Status: implemented; shared by the WASM, C and Python bindings.
Last updated: 2026-09-23

This is task state, not ambient context. Follow `AGENTS.md`; claim one task ID,
record blockers/decisions under it, and check it off only with evidence.

## Bounded contract

The record-model operations and value encoding every binding shares:
parse/write STEP, ids, exact and subtype type queries, read/edit/add/remove
entities, dangling references, stable error codes. No host code.

## Planned file map

- `src/lib.rs`: crate contract and re-exports
- `src/error.rs`: `BindingError` and stable codes
- `src/value.rs`, `src/value/tagged.rs`: `Kind`, `Tagged`, `MAX_NESTING`
- `src/model.rs`: `IfcModel`
- `src/model/tests.rs`: native tests

## Work queue

- [x] `BCORE-EXTRACT` - move host-independent code out of openbim-ifc-wasm
- [x] `BCORE-FINITE` - reject non-finite reals for every host, not just JS
- [x] `BCORE-SEND` - pin `IfcModel: Send + Sync` at compile time
- [ ] `BCORE-DOMAIN` - opt-in domain views (properties, quantities) once a
  binding consumer asks; facade first, then here

## Completion log

- `BCORE-EXTRACT` - moved with `git mv` from openbim-ifc-wasm (history kept);
  13 native tests pass; WASM Node suites (53) unchanged and green.
- `BCORE-FINITE` - NaN/inf were only refused in the JS layer; now
  `Tagged::into_value` refuses them, tested natively and from C and Python.
