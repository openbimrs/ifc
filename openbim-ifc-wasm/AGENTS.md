# openbim-ifc-wasm instructions

Purpose: WebAssembly bindings for the `openbim-ifc` facade, so JavaScript
and TypeScript can read, edit and write IFC STEP files (#34, ADR 0013).

Follow `../AGENTS.md`. Read `PLAN.md` only for assigned implementation or
roadmap work; keep progress, blockers, and evidence there.

## Boundary

- Production dependencies: `openbim-ifc-binding-core`, `wasm-bindgen`,
  `js-sys`. Never the facade or an `ifc-*` crate directly;
  `ifc-model/tests/package_architecture.rs` enforces this.
- JS glue only. Model operations, the tagged encoding and its validation
  live in `../openbim-ifc-binding-core`, shared with the C and Python
  bindings; change them there so all three hosts stay identical.

## Layout

- `src/value.rs`: `Tagged` <-> JS objects.
- `src/model.rs`: the `#[wasm_bindgen]` `IfcModel`, a newtype over the
  core model (wasm-bindgen cannot export a foreign type).
- `src/error.rs`: `BindingError` -> JS `IfcError` with its `code`.
- `src/model/types.rs` (wasm32 only): TypeScript `IfcValue` declarations.
- `npm/package.json`: npm manifest; its version must equal `Cargo.toml`'s.
- `tests/js/`: Node smoke and corpus round-trip suites.

## Invariants

- Every `Value` round-trips: `$`/`*`, `.U.`/`.F.`, integer/real, typed
  wrappers and 64-bit integers (`bigint`) stay distinct.
- A refused JS input is an `IfcError` with a stable `code` and leaves the
  model unchanged; never a panic, never a coerced value.
- Anything testable without a JS host belongs in the core, where
  `cargo test -p openbim-ifc-binding-core` covers it natively.

## Verification

```sh
cargo test -p openbim-ifc-binding-core
openbim-ifc-wasm/scripts/build-node-pkg.sh   # wasm build + Node suites
```

`scripts/gate.sh` runs both. The wasm-bindgen CLI must equal the crate pin
exactly; the build script refuses a mismatch and prints the install command.
