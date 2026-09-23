# openbim-ifc-binding-core instructions

Purpose: the host-independent half of every language binding (ADR 0013).
`IfcModel` operations, the lossless `Tagged` value encoding, and
`BindingError` with its stable codes -- written once, shared by
`openbim-ifc-wasm`, `openbim-ifc-capi` and `openbim-ifc-py`.

Follow `../AGENTS.md`. Read `PLAN.md` only for assigned implementation or
roadmap work; keep progress, blockers, and evidence there.

## Boundary

- Production dependencies: `openbim-ifc` only. No host crate (`wasm-bindgen`,
  `pyo3`, `libc`, ...) ever: a host dependency here would make every binding
  pay for every host. `ifc-model/tests/package_architecture.rs` enforces
  that bindings reach IFC only through the facade.
- Glue only. IFC behaviour goes into the facade; this crate may hold only
  what every host shares: the value encoding, input checks that stop the STEP
  writer emitting a malformed file (identifiers, finite reals, nesting), and
  error codes.
- `unsafe` is not allowed here; the only `unsafe` in the workspace is in
  `openbim-ifc-capi`.

## Layout

- `src/model.rs`: `IfcModel`, plain Rust over `ifc::Model`.
- `src/value.rs`, `src/value/tagged.rs`: `Kind`, `Tagged`, `MAX_NESTING`.
- `src/error.rs`: `BindingError` and its stable `code()` strings.
- `src/model/tests.rs`: round-trip, edit and refusal tests.

## Invariants

- `Value -> Tagged` is total; `Tagged -> Value` checks and never panics.
- Every `BindingError` code is part of the public contract of all three
  bindings: add codes, never rename or reuse one.
- `IfcModel` is `Send + Sync` (a compile-time assertion pins it): hosts move
  models across threads.

## Verification

```sh
cargo test -p openbim-ifc-binding-core
```
